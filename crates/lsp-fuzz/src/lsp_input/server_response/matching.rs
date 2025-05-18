use std::collections::HashMap;

use crate::lsp::{
    LspMessage, LspMessageMeta, MessageParam,
    json_rpc::{JsonRPCMessage, MessageId, ResponseError},
    message::{LspResponse, MessageDecodeError, lift_localized_json},
};

#[derive(Debug)]
pub struct RequestResponseMatching<'a> {
    pub responses: HashMap<&'a LspMessage, LspResponse>,
    pub errors: HashMap<&'a LspMessage, ResponseError>,
    pub notifications: Vec<LspMessage>,
    pub requests_from_server: Vec<LspMessage>,
}

impl<'a> RequestResponseMatching<'a> {
    pub fn find_notifications<'n, Notification: LspMessageMeta>(
        &'n self,
    ) -> impl Iterator<Item = &'n Notification::Params>
    where
        Notification::Params: MessageParam<Notification> + 'n,
    {
        self.notifications
            .iter()
            .filter_map(|it| Notification::Params::from_message_ref(it))
    }

    pub fn find_response_of(&self, request: &LspMessage) -> Option<&LspResponse> {
        self.responses.get(request)
    }

    pub(crate) fn match_messages<'rec>(
        sent_messages: impl Iterator<Item = &'a LspMessage>,
        received_messages: impl Iterator<Item = &'rec JsonRPCMessage>,
    ) -> Result<Self, MessageDecodeError> {
        let mut responses = HashMap::new();
        let mut notifications = Vec::new();
        let mut requests_from_server = Vec::new();
        let mut errors = HashMap::new();

        let requests: HashMap<_, _> = sent_messages
            .filter(|it| it.is_request())
            .enumerate()
            .map(|(id, msg)| (MessageId::Number(id + 1), msg))
            .collect();

        for recv in received_messages {
            match recv {
                JsonRPCMessage::Request { method, params, .. } => {
                    let mut params = params.clone();
                    lift_localized_json(&mut params);
                    let request = LspMessage::try_from_json(method, params)?;
                    requests_from_server.push(request);
                }
                JsonRPCMessage::Notification { method, params, .. } => {
                    let mut params = params.clone();
                    lift_localized_json(&mut params);
                    let notification = LspMessage::try_from_json(method, params)?;
                    notifications.push(notification);
                }
                JsonRPCMessage::Response {
                    id: Some(id),
                    result,
                    error,
                    ..
                } => {
                    if let Some(msg) = requests.get(id).copied() {
                        if let Some(result) = result {
                            let mut result = result.clone();
                            lift_localized_json(&mut result);
                            let response = LspResponse::try_from_json(msg.method(), result)?;
                            responses.insert(msg, response);
                        } else if let Some(error) = error {
                            errors.insert(msg, error.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            responses,
            notifications,
            errors,
            requests_from_server,
        })
    }
}
