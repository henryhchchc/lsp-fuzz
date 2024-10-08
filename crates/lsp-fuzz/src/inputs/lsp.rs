


pub enum LspParam {
    
}

// impl<R: Request> std::fmt::Debug for LspRequest<R>
// where
//     R::Params: std::fmt::Debug,
// {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "LspRequest {{ method: {:}, params: {:?} }}",
//             R::METHOD,
//             self.params
//         )
//     }
// }

// impl<R: Request> LspRequest<R> {
//     pub fn new(params: <R as Request>::Params) -> Self {
//         Self {
//             _request: PhantomData,
//             params,
//         }
//     }

//     pub fn try_into_jsonrpc(self, id: usize) -> Result<Vec<u8>, serde_json::Error> {
//         let request_json = json!({
//             "jsonrpc": "2.0",
//             "id": id,
//             "method": R::METHOD,
//             "params": serde_json::to_value(self.params)?
//         });
//         let request_body = serde_json::to_vec(&request_json)?;
//         let content_length = request_body.len();
//         let mut result = format!("Content-Length: {content_length}\r\n\r\n").into_bytes();
//         result.extend(request_body);
//         Ok(result)
//     }
// }

// #[cfg(test)]
// mod test {
//     use lsp_types::request::{Initialize, Request};

//     use super::LspRequest;

//     #[test]
//     fn test_lsp_request() {
//         let request = LspRequest::<Initialize>::new(lsp_types::InitializeParams {
//             workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
//                 uri: "file:///path/to/folder".parse().unwrap(),
//                 name: "folder".to_string(),
//             }]),
//             ..Default::default()
//         });
//         let jsonrpc = request.try_into_jsonrpc(1).unwrap();
//         let header = b"Content-Length: 177\r\n\r\n";
//         assert_eq!(jsonrpc[..header.len()], header[..]);
//         let json_value: serde_json::Value =
//             serde_json::from_slice(&jsonrpc[header.len()..]).unwrap();
//         assert_eq!(json_value["jsonrpc"], "2.0");
//         assert_eq!(json_value["id"], 1);
//         assert_eq!(json_value["method"], Initialize::METHOD);
//         assert!(json_value["params"]["workspaceFolders"][0]["uri"]
//             .as_str()
//             .unwrap()
//             .contains("path/to/folder"));
//     }
// }
