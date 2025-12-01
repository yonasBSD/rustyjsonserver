use std::collections::HashMap;

use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use rustyjsonserver::rjscript::{
    ast::position::Position as RjsPos,
    parser,
    preprocess::lints::{self, error::LintError},
};

struct Backend {
    client: Client,
    docs: RwLock<HashMap<String, String>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: RwLock::new(HashMap::new()),
        }
    }

    fn to_lsp_pos(p: RjsPos) -> Position {
        Position {
            line: p.line.saturating_sub(1) as u32,
            character: p.column.saturating_sub(1) as u32,
        }
    }

    fn single_point_range(p: RjsPos) -> Range {
        let start = Self::to_lsp_pos(p);
        let end = Position {
            line: start.line,
            character: start.character.saturating_add(1),
        };
        Range { start, end }
    }

    fn lint_to_diag(le: &LintError) -> Diagnostic {
        let p = le.pos;
        Diagnostic {
            range: Self::single_point_range(p),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("rjs-lsp".into()),
            code: None,
            code_description: None,
            message: le.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn parse_error_diag(err: &parser::errors::ParseError) -> Diagnostic {
        let p = err.pos();
        Diagnostic {
            range: Self::single_point_range(p),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("rjs-lsp".into()),
            code: None,
            code_description: None,
            message: format!("{err}"),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    async fn analyze_and_publish(&self, uri: Url, text: &str) {
        let diagnostics = match parser::parser::parse_script(text) {
            Ok(block) => {
                let diags: Vec<Diagnostic> = lints::run_lints(&block)
                    .into_iter()
                    .map(|e| Self::lint_to_diag(&e))
                    .collect();

                diags
            }
            Err(err) => vec![Self::parse_error_diag(&err)],
        };

        let _ = self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "rjs-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let _ = self
            .client
            .log_message(MessageType::INFO, "rjs-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let key = uri.to_string();
        let text = params.text_document.text.clone();

        {
            let mut w = self.docs.write().await;
            w.insert(key.clone(), text.clone());
        }

        self.analyze_and_publish(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let key = uri.to_string();

        if let Some(change) = params.content_changes.last() {
            let text = change.text.clone();
            {
                let mut w = self.docs.write().await;
                w.insert(key.clone(), text.clone());
            }
            self.analyze_and_publish(uri, &text).await;
        } else {
            if let Some(text) = self.docs.read().await.get(&key).cloned() {
                self.analyze_and_publish(uri, &text).await;
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let key = uri.to_string();

        if let Some(text) = self.docs.read().await.get(&key).cloned() {
            self.analyze_and_publish(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let key = params.text_document.uri.to_string();
        {
            let mut w = self.docs.write().await;
            w.remove(&key);
        }
        let _ = self
            .client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::build(|client| Backend::new(client)).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
