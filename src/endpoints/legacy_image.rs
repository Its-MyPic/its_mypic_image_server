use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
};

const DEPRECATION_MESSAGE: &str = "\
此舊版API已停用。請使用新版API格式：

舊版格式：/images/<target_file>.<format>
新版格式：/images/<season>/<episode>/<frame>.<format>

範例：
- 舊版：/images/ave-1-2_34567.jpg
- 新版：/images/2/1-2/34567.jpg

詳細說明：
1. season: 1 (一般) 或 2 (ave-)
2. episode: 集數，如 1-2
3. frame: 幀數，如 34567
4. format: 圖片格式 (jpg/jpeg/png/webp)";

pub(crate) async fn handler(
    Path(_target): Path<String>,
) -> impl IntoResponse {
    (StatusCode::GONE, DEPRECATION_MESSAGE)
}
