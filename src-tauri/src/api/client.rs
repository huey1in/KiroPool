use crate::api::interceptor::{
    is_auth_required_url, save_auth_token, AuthInterceptor, Interceptor,
};
use crate::config;
use crate::database::Database;
use reqwest::header::HeaderValue;
use reqwest::{Client, Request, Response};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tracing::error;

/// HTTP 请求客户端，支持拦截器机制
pub struct ApiClient {
    client: Arc<Client>,
    interceptors: Vec<Box<dyn Interceptor>>,
    app_handle: Option<Arc<AppHandle>>,
}

impl ApiClient {
    /// 创建 API 客户端实例
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        // 从配置中读取是否验证SSL证书
        let verify_ssl = config::CONFIG.read().unwrap().api.verify_ssl;

        // 创建客户端构建器并根据配置决定是否验证证书
        let mut client_builder = Client::builder().timeout(config::get_request_timeout());

        // 如果不验证SSL证书，添加跳过验证选项
        if !verify_ssl {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        // 构建HTTP客户端
        let client = Arc::new(
            client_builder
                .build()
                .expect("Failed to create HTTP client"),
        );

        let mut interceptors = Vec::new();
        if let Some(handle) = &app_handle {
            interceptors
                .push(Box::new(AuthInterceptor::new(Arc::new(handle.clone())))
                    as Box<dyn Interceptor>);
        }

        Self {
            client,
            interceptors,
            app_handle: app_handle.map(Arc::new),
        }
    }

    /// 获取基础URL，优先使用inbound配置
    pub fn get_base_url(&self) -> String {
        use crate::api::inbound::get_current_inbound_url;

        // 如果有AppHandle，尝试获取当前线路URL
        if let Some(handle) = &self.app_handle {
            if let Some(db) = handle.try_state::<crate::database::Database>() {
                return get_current_inbound_url(&db);
            }
        }

        // 回退到默认URL
        config::get_default_api_url()
    }

    /// 发送 HTTP 请求
    pub async fn send(&self, mut request: Request) -> Result<Response, reqwest::Error> {
        let url = request.url().to_string();
        let method = request.method().clone();

        // 在请求前添加拦截器处理（如果需要认证）
        if is_auth_required_url(&url) {
            for interceptor in &self.interceptors {
                if interceptor.intercept(&mut request).is_err() {
                    continue;
                }
            }
        }

        // 执行请求
        let result = self.client.execute(request).await;

        match result {
            Ok(response) => {
                self.process_response(response, &method.to_string(), &url)
                    .await
            }
            Err(e) => {
                error!(
                    target: "http_client",
                    "请求失败 - 方法: {}, URL: {}, 错误: {}",
                    method, url, e
                );
                Err(e)
            }
        }
    }

    /// 处理响应
    async fn process_response(
        &self,
        response: Response,
        method: &str,
        url: &str,
    ) -> Result<Response, reqwest::Error> {
        if self.app_handle.is_none() {
            return Ok(response);
        }

        let handle = self.app_handle.as_ref().unwrap();
        let db = handle.state::<Database>();
        let status = response.status();

        let response_text = response.text().await.map_err(|e| {
            error!(
                target: "http_client",
                "读取响应文本失败 - 方法: {}, URL: {}, 状态码: {}, 错误: {}",
                method, url, status, e
            );
            e
        })?;

        if url.contains("/user/updatePassword") {
            if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                if response_json["status"] == 200 {
                    if let Err(e) = crate::api::interceptor::clear_auth_token(&db).await {
                        error!(
                            target: "http_client",
                            "清除认证令牌失败 - URL: {}, 错误: {}",
                            url, e
                        );
                    }
                }
            }
        } else if let Err(e) = save_auth_token(&db, url, &response_text).await {
            error!(
                target: "http_client",
                "保存认证令牌失败 - URL: {}, 错误: {}",
                url, e
            );
        }

        Ok(Response::from(
            http::Response::builder()
                .status(status)
                .body(response_text)
                .unwrap(),
        ))
    }

    /// 创建 GET 请求
    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder {
            inner: self.client.get(url.as_ref()),
            client: self,
        }
    }

    /// 创建 POST 请求
    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder {
            inner: self.client.post(url.as_ref()),
            client: self,
        }
    }

    /// 创建 PUT 请求
    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder {
            inner: self.client.put(url.as_ref()),
            client: self,
        }
    }

    /// 创建 PATCH 请求
    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder {
            inner: self.client.patch(url.as_ref()),
            client: self,
        }
    }

    /// 创建 DELETE 请求
    pub fn delete(&self, url: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder {
            inner: self.client.delete(url.as_ref()),
            client: self,
        }
    }
}

/// HTTP 请求构建器
pub struct RequestBuilder<'a> {
    inner: reqwest::RequestBuilder,
    client: &'a ApiClient,
}

impl<'a> RequestBuilder<'a> {
    /// 发送请求
    pub async fn send(self) -> Result<Response, reqwest::Error> {
        // 在构建请求前获取内部构建器的调试信息
        let debug_info = format!("{:?}", self.inner);

        let request = match self.inner.build() {
            Ok(req) => req,
            Err(e) => {
                error!(
                    target: "http_client",
                    "构建HTTP请求失败 - 请求: {}, 错误: {}",
                    debug_info, e
                );
                return Err(e);
            }
        };
        self.client.send(request).await
    }

    /// 添加表单数据
    pub fn form<T: serde::Serialize + ?Sized>(self, form: &T) -> Self {
        Self {
            inner: self.inner.form(form),
            client: self.client,
        }
    }

    /// 添加 JSON 数据
    pub fn json<T: serde::Serialize + ?Sized>(self, json: &T) -> Self {
        Self {
            inner: self.inner.json(json),
            client: self.client,
        }
    }

    /// 添加请求头
    pub fn header(self, key: &str, value: &str) -> Self {
        Self {
            inner: self
                .inner
                .header(key, HeaderValue::from_str(value).unwrap()),
            client: self.client,
        }
    }

    /// 添加 multipart 表单数据
    pub fn multipart<T: IntoIterator<Item = (String, String)>>(self, form: T) -> Self {
        let mut form_builder = reqwest::multipart::Form::new();
        for (key, value) in form {
            form_builder = form_builder.text(key, value);
        }

        Self {
            inner: self.inner.header("Accept", "*/*").multipart(form_builder),
            client: self.client,
        }
    }
}
