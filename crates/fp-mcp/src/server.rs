//! rmcp-Anbindung: [`FirepixelMcp`] setzt `ServerHandler` um und leitet Werkzeugaufrufe über den
//! [`SessionHost`] an die Sitzung weiter.

use crate::host::{with_session, SessionHost};
use crate::resources;
use crate::tools::{self, Block, ToolEnv};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use std::sync::Arc;

/// Hinweise für Agenten in der `initialize`-Antwort.
pub const INSTRUCTIONS: &str = "Firepixel ist ein Grafikprogramm für Bildschirm- und Webgrafik (Vektor + Bitmap, Seiten, Status, Ebenen, Symbole, FW-PNG). \
Ablauf: new_document oder open → get_document (IDs, Grenzen) → select → run_command (Befehle aus list_commands/describe_command) → render (Bild prüfen) → save/export. \
Alle Änderungen sind rückgängig machbar (undo/history). Befehlsnamen: <menü>.<eintrag> in camelCase (modify.align.left, filter.blur.gaussian) sowie draw.*, element.*, layer.*, page.*, state.*, select.*. \
Koordinaten in Dokumentpixeln, Rechtecke [l, t, r, b], Farben #rrggbb[aa]. Ausführliche Doku: Ressourcen firepixel://docs/agents.md, commands.md, document-model.md, examples.md.";

/// Der MCP-Server (eine Instanz je Verbindung; der Zustand liegt im Host).
#[derive(Clone)]
pub struct FirepixelMcp {
    host: Arc<dyn SessionHost>,
    env: ToolEnv,
}

impl FirepixelMcp {
    /// Neuer Server auf einem Sitzungs-Host.
    pub fn new(host: Arc<dyn SessionHost>) -> FirepixelMcp {
        FirepixelMcp { host, env: ToolEnv::default() }
    }

    /// Mit eigener Werkzeug-Umgebung (z. B. geteilter Renderer, Skripte aus).
    pub fn with_env(host: Arc<dyn SessionHost>, env: ToolEnv) -> FirepixelMcp {
        FirepixelMcp { host, env }
    }

    /// Werkzeugliste im rmcp-Format.
    pub fn tool_list() -> Vec<Tool> {
        tools::tool_defs()
            .into_iter()
            .map(|d| {
                let schema = match d.schema {
                    Value::Object(m) => m,
                    _ => serde_json::Map::new(),
                };
                let ann = ToolAnnotations::new().read_only(d.read_only).destructive(d.destructive).idempotent(d.read_only).open_world(false);
                Tool::new(d.name, d.description, Arc::new(schema)).with_title(d.title).with_annotations(ann)
            })
            .collect()
    }
}

fn to_result(out: tools::ToolOutput) -> CallToolResult {
    let content: Vec<ContentBlock> = out
        .blocks
        .into_iter()
        .map(|b| match b {
            Block::Text(t) => ContentBlock::text(t),
            Block::Image(png) => ContentBlock::image(crate::image::base64(&png), "image/png"),
        })
        .collect();
    let mut r = if out.is_error { CallToolResult::error(content) } else { CallToolResult::success(content) };
    if let Some(v) = out.structured.filter(|v| v.is_object()) {
        r.structured_content = Some(v);
    }
    r
}

impl ServerHandler for FirepixelMcp {
    fn get_info(&self) -> ServerConfig {
        let mut imp = Implementation::new("firepixel", env!("CARGO_PKG_VERSION"));
        imp.title = Some(format!("Firepixel ({})", self.host.mode()));
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().build()).with_server_info(imp).with_instructions(if self.host.mode() == "bridge" {
            format!("{INSTRUCTIONS} Brückenmodus: Du steuerst die laufende Oberfläche, der Nutzer sieht jede Änderung live. screenshot zeigt das ganze Fenster, ui_state den Oberflächenzustand, ui_action löst Oberflächenaktionen aus, ui_find liefert Positionen von Menüeinträgen/Knöpfen/Bedienfeld-Zeilen, ui_input speist echte Maus-/Tastatur-/Datei-Ereignisse ein (wie ein Mensch).")
        } else {
            format!("{INSTRUCTIONS} Headless-Sitzung (screenshot/ui_state/ui_action/ui_input/ui_find nur im Brückenmodus).")
        })
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(FirepixelMcp::tool_list()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        FirepixelMcp::tool_list().into_iter().find(|t| t.name == name)
    }

    async fn call_tool(&self, request: CallToolRequestParams, _context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        if !tools::tool_defs().iter().any(|d| d.name == name) {
            return Err(ErrorData::invalid_params(format!("unbekanntes Werkzeug „{name}“"), None));
        }
        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
        if let Some(req) = tools::ui_request_for(&name, &args) {
            let out = match crate::host::ui_request(self.host.as_ref(), req, tools::ui_timeout(&name, &args)).await {
                Ok(reply) => tools::ui_output(&name, &args, reply),
                Err(e) => tools::error_output(e.to_string()),
            };
            return Ok(to_result(out).into());
        }
        let env = self.env.clone();
        let out = with_session(self.host.as_ref(), move |s| tools::call(s, &env, &name, &args))
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(to_result(out).into())
    }

    async fn list_resources(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListResourcesResult, ErrorData> {
        let list = resources::resources()
            .into_iter()
            .map(|r| Resource::new(r.uri, r.name).with_title(r.title).with_description(r.description).with_mime_type("text/markdown"))
            .collect();
        Ok(ListResourcesResult::with_all_items(list))
    }

    async fn read_resource(&self, request: ReadResourceRequestParams, _context: RequestContext<RoleServer>) -> Result<ReadResourceResponse, ErrorData> {
        let uri = request.uri.clone();
        let text = with_session(self.host.as_ref(), move |s| resources::read(&uri, s.registry()))
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        match text {
            Some(t) => Ok(ReadResourceResult::new(vec![ResourceContents::text(t, request.uri).with_mime_type("text/markdown")]).into()),
            None => Err(ErrorData::invalid_params(format!("unbekannte Ressource {}", request.uri), None)),
        }
    }
}
