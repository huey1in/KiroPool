use super::client::ApiClient;
use super::types::*;
use crate::config;
use crate::database::Database;
use serde_json::json;
use tauri::State;
use tracing::error;

/// 通用API响应处理函数，处理成功和失败情况
async fn handle_api_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    error_context: &str,
) -> Result<ApiResponse<T>, String> {
    // 获取响应文本
    let response_text = response.text().await.map_err(|e| {
        error!(target: "api", "获取{}响应文本失败 - 错误: {}", error_context, e);
        e.to_string()
    })?;

    // 尝试解析为基本JSON格式以获取status和msg
    let api_response: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
        error!(target: "api", "解析{}响应JSON失败 - 错误: {}", error_context, e);
        e.to_string()
    })?;

    // 提取status和msg
    let status = api_response["status"].as_i64().unwrap_or(500) as i32;
    let msg = api_response["msg"]
        .as_str()
        .unwrap_or("未知错误")
        .to_string();

    // 如果status不是200，直接返回错误响应
    if status != 200 {
        return Ok(ApiResponse {
            status,
            msg,
            data: None,
            code: api_response["code"].as_str().map(String::from),
        });
    }

    // 成功情况，尝试解析为完整类型
    match serde_json::from_str::<ApiResponse<T>>(&response_text) {
        Ok(typed_response) => Ok(typed_response),
        Err(e) => {
            error!(target: "api", "解析{}响应为完整类型失败 - 错误: {}", error_context, e);
            // 即使解析失败，依然返回成功状态但data为None
            Ok(ApiResponse {
                status,
                msg,
                data: None,
                code: api_response["code"].as_str().map(String::from),
            })
        }
    }
}

/// 检查用户是否存在
#[tauri::command]
pub async fn check_user(
    client: State<'_, ApiClient>,
    email: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let response = client
        .post(format!("{}/checkUser", config::get_auth_api_url()))
        .form(&[("email", email)])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "检查用户失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "检查用户").await
}

/// 发送验证码
#[tauri::command]
pub async fn send_code(
    client: State<'_, ApiClient>,
    email: String,
    r#type: String,
) -> Result<ApiResponse<SendCodeResponse>, String> {
    let response = client
        .post(format!(
            "{}/register/sendEmailCode",
            config::get_auth_api_url()
        ))
        .form(&[("email", email), ("type", r#type)])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "发送验证码失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "发送验证码").await
}

/// 注册用户
#[tauri::command]
pub async fn register(
    client: State<'_, ApiClient>,
    email: String,
    code: String,
    password: String,
) -> Result<ApiResponse<RegisterResponse>, String> {
    let response = client
        .post(format!("{}/emailRegister", config::get_auth_api_url()))
        .multipart([
            ("email".to_string(), email),
            ("code".to_string(), code),
            ("password".to_string(), password),
            ("spread".to_string(), "0".to_string()),
        ])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "注册用户失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "注册用户").await
}

/// 用户登录
#[tauri::command]
pub async fn login(
    client: State<'_, ApiClient>,
    account: String,
    password: String,
    spread: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    let response = client
        .post(format!("{}/login", config::get_auth_api_url()))
        .form(&[
            ("account", account),
            ("password", password),
            ("spread", spread),
        ])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "登录失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "登录").await
}

/// 获取用户信息
#[tauri::command]
pub async fn get_user_info(client: State<'_, ApiClient>) -> Result<ApiResponse<UserInfo>, String> {
    let response = client
        .get(format!("{}/user", config::get_auth_api_url()))
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "获取用户信息失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "获取用户信息").await
}

/// 激活账户
#[tauri::command]
pub async fn activate(
    client: State<'_, ApiClient>,
    code: String,
) -> Result<ApiResponse<()>, String> {
    let response = client
        .post(format!("{}/user/activate", config::get_auth_api_url()))
        .form(&[("code", code)])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "激活账户失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "激活账户").await
}

/// 修改密码
#[tauri::command]
pub async fn change_password(
    client: State<'_, ApiClient>,
    old_password: String,
    new_password: String,
) -> Result<ApiResponse<()>, String> {
    let response = client
        .post(format!(
            "{}/user/updatePassword",
            config::get_auth_api_url()
        ))
        .form(&[
            ("old_password", old_password.clone()),
            ("new_password", new_password.clone()),
            ("confirm_password", new_password.clone()),
        ])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "修改密码请求失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "修改密码").await
}

/// 获取公告信息
#[tauri::command]
pub async fn get_public_info(
    client: State<'_, ApiClient>,
) -> Result<ApiResponse<PublicInfo>, String> {
    let response = client
        .get(format!("{}/public/info", config::get_auth_api_url()))
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "获取公告信息失败 - 错误: {}", e);
            e.to_string()
        })?;

    response.json().await.map_err(|e| {
        error!(target: "api", "解析公告信息响应失败 - 错误: {}", e);
        e.to_string()
    })
}

/// 重置密码
#[tauri::command]
pub async fn reset_password(
    client: State<'_, ApiClient>,
    email: String,
    code: String,
    password: String,
) -> Result<ApiResponse<()>, String> {
    let response = client
        .post(format!("{}/emailResetPassword", config::get_auth_api_url()))
        .form(&[("email", email), ("code", code), ("password", password)])
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "重置密码请求失败 - 错误: {}", e);
            e.to_string()
        })?;

    handle_api_response(response, "重置密码").await
}

/// 用户登出
#[tauri::command]
pub async fn logout(db: State<'_, Database>) -> Result<ApiResponse<()>, String> {
    db.delete_item("user.info.token").map_err(|e| {
        error!(target: "api", "删除用户token失败 - 错误: {}", e);
        e.to_string()
    })?;

    Ok(ApiResponse {
        status: 200,
        msg: "登出成功".to_string(),
        data: None,
        code: Some("460001".to_string()),
    })
}

/// 设置用户数据
#[tauri::command]
pub async fn set_user_data(
    db: State<'_, Database>,
    key: String,
    value: String,
) -> Result<ApiResponse<()>, String> {
    match db.set_item(&key, &value) {
        Ok(_) => Ok(ApiResponse {
            status: 200,
            msg: "成功设置用户数据".to_string(),
            data: None,
            code: Some("SUCCESS".to_string()),
        }),
        Err(e) => {
            error!(target: "api", "设置用户数据失败 - 键: {}, 错误: {}", key, e);
            Err(e.to_string())
        }
    }
}

/// 获取用户数据
#[tauri::command]
pub async fn get_user_data(
    db: State<'_, Database>,
    key: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match db.get_item(&key) {
        Ok(value) => Ok(ApiResponse {
            status: 200,
            msg: "成功获取用户数据".to_string(),
            data: Some(json!({ "value": value })),
            code: Some("SUCCESS".to_string()),
        }),
        Err(e) => {
            error!(target: "api", "获取用户数据失败 - 键: {}, 错误: {}", key, e);
            Err(e.to_string())
        }
    }
}

/// 删除用户数据
#[tauri::command]
pub async fn del_user_data(
    db: State<'_, Database>,
    key: String,
) -> Result<ApiResponse<()>, String> {
    match db.delete_item(&key) {
        Ok(_) => Ok(ApiResponse {
            status: 200,
            msg: "成功删除用户数据".to_string(),
            data: None,
            code: Some("SUCCESS".to_string()),
        }),
        Err(e) => {
            error!(target: "api", "删除用户数据失败 - 键: {}, 错误: {}", key, e);
            Err(e.to_string())
        }
    }
}

/// 获取公告列表
#[tauri::command]
pub async fn get_article_list(
    client: State<'_, ApiClient>,
) -> Result<ApiResponse<Vec<Article>>, String> {
    // 获取公告数据
    let result = fetch_article_list(&client).await;

    match result {
        Ok(articles) => Ok(ApiResponse {
            status: 200,
            msg: "获取公告成功".to_string(),
            data: Some(articles),
            code: Some("SUCCESS".to_string()),
        }),
        Err(e) => {
            // 接口错误时，返回空列表而不是错误
            error!(target: "api", "获取公告列表失败，返回空列表 - 错误: {}", e);
            Ok(ApiResponse {
                status: 200,
                msg: "获取公告成功".to_string(),
                data: Some(Vec::new()),
                code: Some("SUCCESS".to_string()),
            })
        }
    }
}

/// 内部函数：获取公告列表数据
async fn fetch_article_list(client: &ApiClient) -> Result<Vec<Article>, String> {
    let response = client
        .get(format!("{}/article/list/1", config::get_auth_api_url()))
        .send()
        .await
        .map_err(|e| {
            error!(target: "api", "获取公告列表请求失败 - 错误: {}", e);
            e.to_string()
        })?;

    let response_json: serde_json::Value = response.json().await.map_err(|e| {
        error!(target: "api", "解析公告列表响应失败 - 错误: {}", e);
        e.to_string()
    })?;

    // 检查状态码
    let status = response_json["status"].as_i64().unwrap_or(0);
    if status != 200 {
        let error_msg = "获取公告失败".to_string();
        error!(target: "api", "公告列表状态码错误 - 状态码: {}", status);
        return Err(error_msg);
    }

    // 提取所需字段
    let empty_vec = Vec::new();
    let data = response_json["data"].as_array().unwrap_or(&empty_vec);
    let mut articles = Vec::new();

    for item in data {
        let id = item["id"].as_i64().unwrap_or(0) as i32;
        let title = item["title"].as_str().unwrap_or("").to_string();
        let content = item["content"].as_str().unwrap_or("").to_string();

        articles.push(Article { id, title, content });
    }

    Ok(articles)
}

/// 标记文章为已读
#[tauri::command]
pub async fn mark_article_read(
    db: State<'_, Database>,
    article_id: i32,
) -> Result<ApiResponse<()>, String> {
    // 获取已读ID集合
    let read_ids = match db.get_item("system.articles") {
        Ok(Some(data)) => serde_json::from_str::<Vec<i32>>(&data).unwrap_or_default(),
        Ok(None) => Vec::new(),
        Err(e) => {
            error!(target: "api", "获取已读文章列表失败 - 错误: {}", e);
            Vec::new()
        }
    };

    // 检查文章ID是否已在已读列表中
    let mut updated_ids = read_ids.clone();
    if !updated_ids.contains(&article_id) {
        updated_ids.push(article_id);

        // 保存更新后的已读ID列表
        let json_data = serde_json::to_string(&updated_ids).map_err(|e| {
            error!(target: "api", "序列化已读文章ID列表失败 - 错误: {}", e);
            e.to_string()
        })?;
        db.set_item("system.articles", &json_data).map_err(|e| {
            error!(target: "api", "保存已读文章ID列表失败 - 错误: {}", e);
            e.to_string()
        })?;
    }

    Ok(ApiResponse {
        status: 200,
        msg: "文章已标记为已读".to_string(),
        data: None,
        code: Some("SUCCESS".to_string()),
    })
}
