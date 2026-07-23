use crate::chat_engine::stream_seq::ChatSource;

#[derive(Debug, Clone)]
pub struct LastUserSnapshot {
    pub source: String,
    pub text: String,
    pub attachment_count: usize,
}

pub(crate) struct ImLiveMirrorState;

pub(crate) async fn attach_im_live_mirror(
    _session_id: &str,
    _source: ChatSource,
    last_user: Option<LastUserSnapshot>,
) -> Option<ImLiveMirrorState> {
    if let Some(user) = last_user {
        let _ = (&user.source, &user.text, user.attachment_count);
    }
    None
}

pub(crate) async fn attach_im_injection_mirror(_session_id: &str) -> Option<ImLiveMirrorState> {
    None
}

pub(crate) async fn finalize_im_live_mirror(_state: ImLiveMirrorState, _response: &str) {}

pub(crate) async fn abort_im_live_mirror_with_body(
    _state: ImLiveMirrorState,
    _body: Option<String>,
) {
}
