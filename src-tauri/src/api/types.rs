use serde::{Deserialize, Serialize};

// RESTful API响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

// 注册响应
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub token: String,
    #[serde(rename = "expires_time")]
    pub expires_time: i64,
}

// 登录响应
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user_info: Option<UserInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendCodeResponse {
    #[serde(default)]
    pub code: Option<String>,
}

// 用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    // 用户总额度
    #[serde(rename = "totalCredits")]
    pub total_credits: i32,
    // 已使用额度
    #[serde(rename = "usedCredits")]
    pub used_credits: i32,
    #[serde(rename = "creditBalance")]
    pub credit_balance: i32,
    // 过期时间
    #[serde(rename = "expireTime")]
    pub expire_time: String,
    // 用户等级
    pub level: i32,
    // 是否已过期
    #[serde(rename = "isExpired")]
    pub is_expired: bool,
    // 用户名
    pub username: String,
    // 用户级别文本
    #[serde(rename = "code_level")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub code_level: String,
    // 激活码状态
    #[serde(rename = "code_status")]
    #[serde(default)]
    pub code_status: i32,
}

// 账户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    // 账户ID
    pub id: i32,
    // 账户名
    pub account: String,
    // 密码
    pub password: String,
    // 令牌
    pub token: String,
    // 使用次数
    #[serde(rename = "usage_count")]
    pub usage_count: i32,
    // 状态
    pub status: i32,
    // 创建时间
    #[serde(rename = "create_time")]
    pub create_time: String,
    // 分配时间
    #[serde(rename = "distributed_time")]
    pub distributed_time: String,
    // 更新时间
    #[serde(rename = "update_time")]
    pub update_time: String,
}

// 账户详情
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountDetail {
    pub email: String,
    pub token: String,
}

// 登录请求
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
    pub spread: String,
}

// 检查用户请求
#[derive(Debug, Serialize)]
pub struct CheckUserRequest {
    pub email: String,
}

// 发送验证码请求
#[derive(Debug, Serialize)]
pub struct SendCodeRequest {
    pub email: String,
    pub r#type: String, // register或reset
}

// 注册请求
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub code: String,
    pub password: String,
    pub spread: String,
}

// 重置密码请求
#[derive(Debug, Serialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub password: String,
}

// 激活请求
#[derive(Debug, Serialize)]
pub struct ActivateRequest {
    pub code: String,
}

// 激活响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivateResponse {
    #[serde(rename = "expireTime")]
    pub expire_time: i64,
    pub level: i32,
}

// 修改密码请求
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordChangeRequest {
    #[serde(rename = "oldPassword")]
    pub old_password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
    #[serde(rename = "confirmPassword")]
    pub confirm_password: String,
}

// 公告信息
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicInfo {
    pub r#type: String,
    pub closeable: bool,
    pub props: PublicInfoProps,
    pub actions: Vec<PublicInfoAction>,
}

// 公告属性
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicInfoProps {
    pub title: String,
    pub description: String,
}

// 公告动作
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicInfoAction {
    pub r#type: String,
    pub text: String,
    pub url: String,
}

// 公告数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Article {
    pub id: i32,
    pub title: String,
    pub content: String,
}

// 公告列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleListResponse {
    pub articles: Vec<Article>,
}

#[cfg(test)]
mod send_code_response_tests {
    use super::*;

    #[test]
    fn deserializes_development_verification_code() {
        let response: ApiResponse<SendCodeResponse> =
            serde_json::from_str(r#"{"status":200,"msg":"ok","data":{"code":"123456"}}"#)
                .expect("send-code response should deserialize");

        assert_eq!(response.data.unwrap().code.as_deref(), Some("123456"));
    }
}
