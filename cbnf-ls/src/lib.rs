use std::fmt::Display;

use cbnf::Cbnf;
use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

// TODO: consider parsing newlines at a different point in the process
// Add LSpan (Line Span) to cbnf

// TODO: add more fail fast parsing, for eg:
// fail fast when neither semi or brace follows a rule/meta name

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub forms: DashMap<Url, Document>,
}

#[derive(Debug, Default)]
pub struct Document {
    pub tokens: Cbnf,
    pub source: String,
    pub line_breaks: Vec<usize>,
}

fn find_lines(source: &str) -> Vec<usize> {
    source
        .bytes()
        .enumerate()
        .filter_map(|(i, b)| (b == b'\n').then_some(i))
        .collect()
}

/// returns `Some((true, ..))` if target is first line
const fn find_line(lbs: &[usize], target: usize) -> usize {
    let mut i = 0;
    loop {
        if i >= lbs.len() {
            break i;
        }
        if lbs[i] > target {
            break i;
        }
        i += 1;
    }
}

impl Document {
    #[must_use]
    pub fn new(source: String) -> Self {
        let tokens = Cbnf::parse(&source);
        let line_breaks = find_lines(&source);
        Self {
            tokens,
            source,
            line_breaks,
        }
    }

    fn get_location(&self, uri: Url, span: cbnf::span::BSpan) -> Location {
        let line_from = find_line(&self.line_breaks, span.from);
        let line_to = find_line(&self.line_breaks, span.to);
        let from = match line_from {
            0 => span.from,
            _ => span.from - self.line_breaks[line_from - 1],
        };
        let to = match line_to {
            0 => span.to,
            _ => span.to - self.line_breaks[line_to - 1],
        };
        Location {
            uri,
            range: Range {
                start: Position {
                    line: u32::try_from(line_from).unwrap(),
                    character: u32::try_from(from).unwrap().saturating_sub(1),
                },
                end: Position {
                    line: u32::try_from(line_to).unwrap(),
                    character: u32::try_from(to).unwrap().saturating_sub(1),
                },
            },
        }
    }
}

impl Backend {
    #[must_use]
    pub fn new(client: Client) -> Self {
        let forms = DashMap::new();
        Self { client, forms }
    }
    async fn info(&self, m: impl Display + Send) {
        tracing::info!("{m}");
        self.client.log_message(MessageType::INFO, m).await;
    }
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
        document_symbol_provider: Some(OneOf::Left(true)),
        position_encoding: Some(PositionEncodingKind::UTF8),
        ..Default::default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    // TODO: consider making 'syntax.cbnf' be the root of a file.
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: capabilities(),
            server_info: Some(ServerInfo {
                name: "cbnf-ls".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(doc) = self.forms.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let uri = params.text_document.uri;
        #[allow(deprecated)]
        let items = doc
            .tokens
            .rules()
            .iter()
            .map(|(name, rule)| SymbolInformation {
                name: name.to_owned(),
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                location: doc.get_location(uri.clone(), rule.name),
                container_name: None,
            })
            .collect::<Vec<_>>();
        Ok((!items.is_empty()).then_some(DocumentSymbolResponse::Flat(items)))
    }
    async fn initialized(&self, _: InitializedParams) {
        self.info("cbnf-ls initialized").await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.info("cbnf-ls shutdown").await;
        Ok(())
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let Some(i) = params
            .content_changes
            .iter()
            .enumerate()
            .find_map(|(i, changes)| changes.range.is_none().then_some(i))
        else {
            self.info("client inputted invalid change data").await;
            return;
        };
        let src = params.content_changes.swap_remove(i).text;
        let doc = Document::new(src);
        *self.forms.entry(params.text_document.uri).or_default() = doc;
    }

    // TODO: do something with this
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let _ = params;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.forms.insert(
            params.text_document.uri,
            Document::new(params.text_document.text),
        );
    }
}
