use std::fmt::Display;

use cbnf::Cbnf;
use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub forms: DashMap<Url, (Cbnf, String)>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let forms = DashMap::new();
        Self { client, forms }
    }
    async fn info(&self, m: impl Display) {
        tracing::info!("{m}");
        self.client.log_message(MessageType::INFO, m).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                definition_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["window/logMessage".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cbnf-ls".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        self.info("cbnf-ls initialized").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.info(format!("did_change: {params:#?}")).await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.info(format!("did_change: {}", params.settings)).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.info(format!("did_open: {}", params.text_document.uri))
            .await;
        let src = params.text_document.text;
        let parse = Cbnf::parse(&src);
        self.forms.insert(params.text_document.uri, (parse, src));
    }
}
