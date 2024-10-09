#![allow(clippy::significant_drop_tightening)]
use std::fmt::Display;

use cbnf::indexmap::IndexMap;
use cbnf::util::valid_id;
use cbnf::{span::BSpan, Cbnf, Rule, Term};
use dashmap::DashMap;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};

// TODO: consider parsing newlines at a different point in the process
// Add LSpan (Line Span) to cbnf

// TODO: add more fail fast parsing, for eg:
// fail fast when neither semi or brace follows a rule/meta name

// TODO: hide all of the below behind another layer which can be tested

// TODO: create better diagnostics messages, find common errors

// TODO: try to precompute more things

// TODO: try to create a set of references at parse time

// TODO: try to update the state with partial changes instead
// of completely recomputing it each time

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub forms: DashMap<Url, Document>,
}

impl Backend {
    fn get_doc(&self, uri: &Url) -> Result<dashmap::mapref::one::Ref<'_, Url, Document>> {
        self.forms.get(uri).map_or_else(unknown_uri, Ok)
    }
}

#[allow(unused)]
#[derive(Debug, Default)]
pub struct Document {
    source: String,
    line_breaks: Vec<u32>,
    rules: IndexMap<String, Rule>,
    comments: Vec<cbnf::Comment>,
    docs: Vec<cbnf::DocComment>,
    errors: Vec<cbnf::parser::error::Error>,
    terms: Vec<Term>,
    cache: Cache,
}

#[derive(Debug, Default)]
pub struct Cache {
    diagnostics: Vec<Diagnostic>,
    completions: Vec<CompletionItem>,
}

fn find_lines(source: &str) -> Vec<u32> {
    let mut lines = Vec::new();
    let mut i = 0;
    while (i as usize) < source.len() {
        if source.as_bytes()[i as usize] == b'\n' {
            lines.push(i);
        }
        i += 1;
    }
    lines
}

/// returns `Some((true, ..))` if target is first line
const fn find_line(lbs: &[u32], target: u32) -> u32 {
    let mut i = 0;
    loop {
        if i as usize >= lbs.len() {
            break i;
        }
        if lbs[i as usize] > target {
            break i;
        }
        i += 1;
    }
}

const fn get_range(breaks: &[u32], span: BSpan) -> Range {
    let line_from = find_line(breaks, span.from);
    let line_to = find_line(breaks, span.to);
    let from = match line_from {
        0 => span.from,
        _ => span.from - breaks[line_from as usize - 1],
    };
    let to = match line_to {
        0 => span.to,
        _ => span.to - breaks[line_to as usize - 1],
    };
    Range {
        start: Position {
            line: line_from,
            character: from.saturating_sub(1),
        },
        end: Position {
            line: line_to,
            character: to.saturating_sub(1),
        },
    }
}

impl Document {
    #[must_use]
    fn new(source: String) -> Self {
        let tokens = Cbnf::parse(&source);
        let line_breaks = find_lines(&source);
        let loose_terms = tokens.terms.iter().filter_map(|t| match t {
            &Term::Ident(span)
                if !tokens.rules().contains_key(span.slice(&source))
                    && !is_keyword(span.slice(&source)) =>
            {
                Some(Diagnostic {
                    range: get_range(&line_breaks, span),
                    message: "Unknown term".into(),
                    ..Default::default()
                })
            }
            _ => None,
        });
        let diagnostics = tokens
            .errors
            .iter()
            .map(|e| Diagnostic {
                range: get_range(&line_breaks, e.span()),
                message: e.message(),
                ..Default::default()
            })
            .chain(loose_terms)
            .collect();
        let completions = tokens
            .rules
            .iter()
            .map(|(n, _)| CompletionItem {
                label: n.to_owned(),
                kind: Some(CompletionItemKind::CLASS),
                ..Default::default()
            })
            .collect();
        Self {
            source,
            line_breaks,
            rules: tokens.rules,
            comments: tokens.comments,
            docs: tokens.docs,
            terms: tokens.terms,
            errors: tokens.errors,
            cache: Cache {
                diagnostics,
                completions,
            },
        }
    }

    fn references<'a>(&'a self, name: &'a str) -> impl Iterator<Item = BSpan> + 'a {
        self.terms
            .iter()
            .filter_map(move |t| (t.span().slice(&self.source) == name).then_some(t.span()))
    }

    fn get_rule(&self, pos: u32) -> Option<Rule> {
        use std::cmp::Ordering::*;
        let pos = self
            .rules
            .binary_search_by(|_, r| match (pos >= r.name.from, pos < r.name.to) {
                // if the position lies within a group, go to the item within the group
                (true, true) => Equal,
                (true, false) => Less,
                (false, true) => Greater,
                (false, false) => unreachable!(),
            })
            .ok()?;
        Some(self.rules[pos])
    }

    fn get_token(&self, pos: u32) -> Option<Term> {
        use std::cmp::Ordering::*;
        let pos = self
            .terms
            .binary_search_by(|t| match (pos >= t.span().from, pos < t.span().to) {
                // if the position lies within a group, go to the item within the group
                (true, true) if matches!(t, Term::Or(_) | Term::Group(_)) => Less,
                (true, true) => Equal,
                (true, false) => Less,
                (false, true) => Greater,
                (false, false) => unreachable!(),
            })
            .ok()?;
        Some(self.terms[pos])
    }

    fn get_point(&self, pos: Position) -> u32 {
        if pos.line == 0 {
            pos.character
        } else {
            self.line_breaks[pos.line as usize - 1] + pos.character + 1
        }
    }

    fn get_range(&self, span: BSpan) -> Range {
        get_range(&self.line_breaks, span)
    }
}

fn is_keyword(source: &str) -> bool {
    matches!(source, "nil")
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
        text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
        document_symbol_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        position_encoding: Some(PositionEncodingKind::UTF8),
        rename_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["$".into()]),
            ..Default::default()
        }),
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
            DiagnosticOptions::default(),
        )),
        ..Default::default()
    }
}

fn unknown_uri<T>() -> Result<T> {
    Err(tower_lsp::jsonrpc::Error::invalid_params(
        "unknown uri inputted",
    ))
}

// TODO: consider returning an error on nonexistant doc uris

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

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: self
                        .get_doc(&params.text_document.uri)?
                        .cache
                        .diagnostics
                        .clone(),
                },
            }),
        ))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(
            self.get_doc(&params.text_document_position.text_document.uri)?
                .cache
                .completions
                .clone(),
        )))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        if !valid_id(&params.new_name) {
            return Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Invalid Name Inputted",
            ));
        }
        let uri = params.text_document_position.text_document.uri;
        let doc = self.get_doc(&uri)?;
        let pos = doc.get_point(params.text_document_position.position);
        let (span, rule) = match doc.get_rule(pos) {
            Some(r) => (r.name, true),
            None => match doc.get_token(pos) {
                Some(Term::Ident(span)) => (span, false),
                _ => return Ok(None),
            },
        };
        let mut edits: Vec<_> = doc
            .references(span.slice(&doc.source))
            .chain(rule.then_some(span))
            .map(|span| TextEdit {
                range: doc.get_range(span),
                new_text: params.new_name.clone(),
            })
            .collect();
        if span.slice(&doc.source).starts_with('$') {
            for te in &mut edits {
                te.range.start.character += 1;
                if te.range.end.line != te.range.start.line {
                    // NOTE: stops eol from being cut
                    te.range.end.line = te.range.start.line;
                    te.range.end.character = te.range.start.character + (span.to - span.from);
                }
            }
        };
        Ok(Some(WorkspaceEdit {
            changes: Some([(uri, edits)].into()),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let doc = self.get_doc(&uri)?;
        let pos = params.text_document_position.position;
        let pos = doc.get_point(pos);
        let span = match doc.get_rule(pos) {
            Some(r) => r.name,
            None => match doc.get_token(pos) {
                Some(Term::Ident(span)) => span,
                _ => return Ok(None),
            },
        };
        let refs = doc
            .references(span.slice(&doc.source))
            .map(|span| Location {
                uri: uri.clone(),
                range: doc.get_range(span),
            })
            .collect();
        Ok(Some(refs))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let doc = self.get_doc(&uri)?;
        let pos = params.text_document_position_params.position;
        let pos = doc.get_point(pos);
        let Some(Term::Ident(span)) = doc.get_token(pos) else {
            return Ok(None);
        };
        let Some(def) = doc.rules.get(span.slice(&doc.source)) else {
            return Ok(None);
        };
        let loc = Location {
            uri: uri.clone(),
            range: doc.get_range(def.name),
        };
        Ok(Some(GotoDefinitionResponse::Scalar(loc)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.goto_definition(params).await
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let doc = self.get_doc(&uri)?;
        #[allow(deprecated)]
        let items = doc
            .rules
            .iter()
            .map(|(name, rule)| SymbolInformation {
                name: name.to_owned(),
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: doc.get_range(rule.name),
                },
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

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.forms.remove(&params.text_document.uri);
    }
}
