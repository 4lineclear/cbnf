use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let server_info = Some(ServerInfo {
            name: "cbnf-ls".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        });
        let text_document_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
        let capabilities = ServerCapabilities {
            text_document_sync,
            ..Default::default()
        };
        Ok(InitializeResult {
            server_info,
            capabilities,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cbnf-ls initialize")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let _ = params;
    }
}
