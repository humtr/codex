//! Termux changes at upstream operation boundaries, independent of private names
//! and rustfmt layout. Source spans preserve every unrelated byte.

mod permissions;

use super::{Result, DESCRIPTION, FULL_ACCESS, WORKSPACE};
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::BTreeSet;
use std::ops::Range;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ItemFn, Lit, Member};

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn leaf(expr: &Expr) -> Option<String> {
    if let Expr::Path(path) = expr {
        path.path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
    } else {
        None
    }
}

fn literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(value) => match &value.lit {
            Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        _ => None,
    }
}

fn test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || attr
                .path()
                .segments
                .last()
                .is_some_and(|s| s.ident == "test")
            || (attr.path().is_ident("cfg") && tokens(attr) == "# [cfg (test)]")
    })
}

#[derive(Default)]
pub(super) struct Nodes<'a> {
    pub functions: Vec<(&'a syn::Ident, Span)>,
    exprs: Vec<&'a Expr>,
    items: Vec<&'a ItemFn>,
    statements: Vec<&'a syn::Stmt>,
    constants: Vec<&'a syn::ItemConst>,
}

impl<'a> Visit<'a> for Nodes<'a> {
    fn visit_item_mod(&mut self, item: &'a syn::ItemMod) {
        if !test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'a ItemFn) {
        if !test(&item.attrs) {
            self.functions.push((&item.sig.ident, item.span()));
            self.items.push(item);
            visit::visit_item_fn(self, item);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'a syn::ImplItemFn) {
        if !test(&item.attrs) {
            self.functions.push((&item.sig.ident, item.span()));
            visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_stmt(&mut self, statement: &'a syn::Stmt) {
        self.statements.push(statement);
        visit::visit_stmt(self, statement);
    }
    fn visit_item_const(&mut self, item: &'a syn::ItemConst) {
        if !test(&item.attrs) {
            self.constants.push(item);
            visit::visit_item_const(self, item);
        }
    }
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.exprs.push(expr);
        visit::visit_expr(self, expr);
    }
}

pub(super) fn parse(text: &str) -> Result<syn::File> {
    syn::parse_file(text).map_err(|error| format!("Rust source syntax: {error}"))
}

pub(super) fn nodes(file: &syn::File) -> Nodes<'_> {
    let mut nodes = Nodes::default();
    nodes.visit_file(file);
    nodes
}

fn within(inner: Span, outer: Span) -> bool {
    let inner = inner.byte_range();
    let outer = outer.byte_range();
    outer.start <= inner.start && inner.end <= outer.end
}

struct Edits<'a> {
    text: &'a str,
    edits: Vec<(Range<usize>, String)>,
}
impl<'a> Edits<'a> {
    fn set(&mut self, span: Span, value: impl Into<String>) {
        self.edits.push((span.byte_range(), value.into()));
    }
    fn insert(&mut self, at: usize, value: impl Into<String>) {
        self.edits.push((at..at, value.into()));
    }
    fn source(&self, node: &impl Spanned) -> &'a str {
        &self.text[node.span().byte_range()]
    }
    fn finish(mut self) -> Result<String> {
        self.edits.sort_by_key(|(range, _)| range.start);
        let mut text = self.text.to_owned();
        let mut boundary = text.len();
        for (range, value) in self.edits.into_iter().rev() {
            if range.end > boundary {
                return Err("overlapping Rust source adaptations".into());
            }
            text.replace_range(range.clone(), &value);
            boundary = range.start;
        }
        parse(&text)?;
        Ok(text)
    }
}

pub(super) fn profile_resolvers(text: &str) -> Result<BTreeSet<String>> {
    let file = parse(text)?;
    Ok(nodes(&file)
        .items
        .iter()
        .filter(|item| tokens(&item.sig.output).contains("LoaderOverrides"))
        .map(|item| item.sig.ident.to_string())
        .collect())
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Operation {
    ForegroundProfile,
    ProfileAdmission,
    Protoc,
    SocketRoot,
    ApprovalPreset,
    AutoReviewMode,
    ResumePermissions,
    PermissionShortcut,
    PermissionMatch,
    PermissionSelection,
    PermissionDescription,
    PermissionMenu,
    AutoReviewDescription,
}

impl Operation {
    pub fn package(self) -> &'static str {
        match self {
            Self::ForegroundProfile | Self::ProfileAdmission => "codex-cli",
            Self::Protoc => "codex-code-mode-protocol",
            Self::SocketRoot => "codex-uds",
            Self::ApprovalPreset => "codex-utils-approval-presets",
            _ => "codex-tui",
        }
    }
    pub fn anchor(self) -> &'static str {
        match self {
            Self::ForegroundProfile => "run_main_with_transport_options",
            Self::ProfileAdmission => "Subcommand",
            Self::Protoc => "protoc_executable",
            Self::SocketRoot => "canonicalize",
            Self::ApprovalPreset => "ApprovalPreset",
            Self::AutoReviewMode => "AutoReviewMode",
            Self::ResumePermissions => "ThreadParamsMode",
            Self::PermissionShortcut => "matches",
            Self::PermissionMatch => "full-access",
            Self::PermissionSelection => ":ask-for-approval",
            Self::PermissionDescription => "auto",
            Self::PermissionMenu => ":read-only",
            Self::AutoReviewDescription => "Only ask for actions detected as potentially unsafe",
        }
    }
    pub fn adapt(self, text: &str, resolvers: &BTreeSet<String>) -> Result<Option<String>> {
        let file = parse(text)?;
        // Builtin vec! stores Rust expressions in macro tokens. Parse its array
        // grammar once, then visit the original expression spans as usual.
        let mut elements = Vec::new();
        for expr in &nodes(&file).exprs {
            if let Expr::Macro(mac) = expr {
                if mac.mac.path.is_ident("vec") {
                    let content = &mac.mac.tokens;
                    let array = syn::parse2::<Expr>(quote::quote!([#content]))
                        .map_err(|error| format!("vector expression syntax: {error}"))?;
                    elements.push(array);
                }
            }
        }
        let mut nodes = nodes(&file);
        for element in &elements {
            nodes.visit_expr(element);
        }
        let mut edits = Edits {
            text,
            edits: Vec::new(),
        };
        let mut found = 0;
        match self {
            Self::ForegroundProfile => {
                for (_, scope) in &nodes.functions {
                    let calls: Vec<_> = nodes
                        .exprs
                        .iter()
                        .filter_map(|expr| match expr {
                            Expr::Call(call) if within(call.span(), *scope) => Some(call),
                            _ => None,
                        })
                        .collect();
                    let targets: Vec<_> = calls
                        .iter()
                        .filter(|call| {
                            matches!(
                                leaf(&call.func).as_deref(),
                                Some("print_startup_config" | "run_main_with_transport_options")
                            )
                        })
                        .collect();
                    if !["print_startup_config", "run_main_with_transport_options"]
                        .iter()
                        .all(|operation| {
                            targets
                                .iter()
                                .any(|call| leaf(&call.func).as_deref() == Some(operation))
                        })
                    {
                        continue;
                    }
                    let templates: BTreeSet<_> = nodes
                        .exprs
                        .iter()
                        .filter_map(|expr| match expr {
                            Expr::Try(tried) if within(tried.span(), *scope) => {
                                let Expr::Call(call) = &*tried.expr else {
                                    return None;
                                };
                                (leaf(&call.func).is_some_and(|name| resolvers.contains(&name))
                                    && tokens(&call.args).contains("config_profile_v2"))
                                .then(|| edits.source(tried))
                            }
                            _ => None,
                        })
                        .collect();
                    if templates.len() != 1 {
                        return Err(
                            "foreground app-server: ambiguous profile resolver/data source".into(),
                        );
                    }
                    let template = *templates.first().unwrap();
                    for call in targets {
                        let candidates: Vec<_> = call
                            .args
                            .iter()
                            .filter(|arg| {
                                tokens(*arg) == "LoaderOverrides :: default ()"
                                    || tokens(*arg)
                                        == tokens(&syn::parse_str::<Expr>(template).unwrap())
                            })
                            .collect();
                        if candidates.len() != 1 {
                            return Err("foreground app-server: loader contract changed".into());
                        }
                        let arg = candidates[0];
                        edits.set(arg.span(), template);
                    }
                    found += 1;
                }
            }
            Self::ProfileAdmission => {
                for expr in &nodes.exprs {
                    let Expr::Match(matched) = expr else {
                        continue;
                    };
                    let Some(arm) = matched.arms.iter().find(|arm| {
                        let pat = tokens(&arm.pat);
                        pat.contains("Subcommand :: Exec") && pat.contains("Subcommand :: Debug")
                    }) else {
                        continue;
                    };
                    let body = edits.source(&*arm.body);
                    let returned: ExprCall = syn::parse_str(body)
                        .map_err(|_| "profile admission: expected successful optional profile")?;
                    if leaf(&returned.func).as_deref() != Some("Ok") || returned.args.len() != 1 {
                        return Err("profile admission: return contract changed".into());
                    }
                    if !matched
                        .arms
                        .iter()
                        .any(|arm| tokens(&arm.pat).contains("Subcommand :: AppServer"))
                    {
                        edits.insert(arm.span().byte_range().start,
                            format!("Subcommand::AppServer(server) if server.subcommand.is_none() => {body},\n"));
                    }
                    found += 1;
                }
            }
            Self::Protoc => {
                for expr in &nodes.exprs {
                    let Expr::MethodCall(call) = expr else {
                        continue;
                    };
                    if call.method != "protoc_executable" {
                        continue;
                    }
                    let arg = call
                        .args
                        .first()
                        .filter(|_| call.args.len() == 1)
                        .ok_or("protoc: executable argument changed")?;
                    if tokens(arg) != "protoc_bin_vendored :: protoc_bin_path () ?" {
                        return Err("protoc: unsupported executable selection contract".into());
                    }
                    edits.set(arg.span(), "match std::env::var_os(\"PROTOC\") { Some(path) => std::path::PathBuf::from(path), None => protoc_bin_vendored::protoc_bin_path()?, }");
                    let (_, scope) = nodes
                        .functions
                        .iter()
                        .find(|(_, span)| within(call.span(), *span))
                        .ok_or("protoc: build scope missing")?;
                    // Place the notification inside the same build function, before its result.
                    let function = nodes
                        .items
                        .iter()
                        .find(|item| item.span().byte_range() == scope.byte_range())
                        .ok_or("protoc: build function missing")?;
                    edits.insert(
                        function.block.brace_token.span.open().byte_range().end,
                        "\nprintln!(\"cargo:rerun-if-env-changed=PROTOC\");\n",
                    );
                    found += 1;
                }
            }
            Self::SocketRoot => {
                for expr in &nodes.exprs {
                    let Expr::Call(call) = expr else {
                        continue;
                    };
                    if leaf(&call.func).as_deref() != Some("canonicalize")
                        || call.args.len() != 1
                        || literal(&call.args[0]).as_deref() != Some("/tmp")
                    {
                        continue;
                    }
                    let (_, scope) = nodes
                        .functions
                        .iter()
                        .find(|(_, span)| within(call.span(), *span))
                        .ok_or("UDS root scope missing")?;
                    let names: Vec<_> = nodes
                        .exprs
                        .iter()
                        .filter_map(|expr| match expr {
                            Expr::Macro(mac)
                                if within(mac.span(), *scope)
                                    && mac.mac.path.is_ident("format") =>
                            {
                                syn::parse2::<syn::LitStr>(mac.mac.tokens.clone())
                                    .ok()
                                    .filter(|s| s.value().starts_with("codex-daemon-"))
                            }
                            _ => None,
                        })
                        .collect();
                    if names.len() != 1 || !nodes.exprs.iter().any(|expr| within(expr.span(), *scope) && matches!(expr, Expr::Call(c) if leaf(&c.func).as_deref() == Some("geteuid"))) {
                        return Err("UDS root: fixed per-user rendezvous contract changed".into());
                    }
                    edits.set(call.args[0].span(), "\"/proc/self/fd/35\"");
                    edits.set(
                        names[0].span(),
                        format!(
                            "{:?}",
                            names[0].value().strip_prefix("codex-daemon-").unwrap()
                        ),
                    );
                    found += 1;
                }
            }
            Self::ApprovalPreset => {
                for expr in &nodes.exprs {
                    let Expr::Struct(preset) = expr else {
                        continue;
                    };
                    if preset
                        .path
                        .segments
                        .last()
                        .is_none_or(|s| s.ident != "ApprovalPreset")
                        || !preset.fields.iter().any(|field| {
                            field.member == Member::Named(syn::Ident::new("id", Span::call_site()))
                                && literal(&field.expr).as_deref() == Some("auto")
                        })
                    {
                        continue;
                    }
                    let field = |name: &str| {
                        preset
                            .fields
                            .iter()
                            .find(|field| matches!(&field.member, Member::Named(id) if id == name))
                            .ok_or_else(|| format!("auto preset: missing {name}"))
                    };
                    if tokens(&field("approval")?.expr) != "AskForApproval :: OnRequest" {
                        return Err("auto preset: upstream approval policy changed".into());
                    }
                    let profile = &field("active_permission_profile")?.expr;
                    if !tokens(profile).contains(WORKSPACE) {
                        return Err("auto preset: upstream active profile changed".into());
                    }
                    edits.set(
                        profile.span(),
                        edits.source(profile).replace(WORKSPACE, FULL_ACCESS),
                    );
                    if tokens(&field("permission_profile")?.expr)
                        != "PermissionProfile :: workspace_write ()"
                    {
                        return Err("auto preset: upstream sandbox contract changed".into());
                    }
                    edits.set(
                        field("permission_profile")?.expr.span(),
                        "PermissionProfile::Disabled",
                    );
                    edits.set(
                        field("description")?.expr.span(),
                        format!("{DESCRIPTION:?}"),
                    );
                    found += 1;
                }
            }
            _ => {
                found = permissions::adapt(self, &nodes, &mut edits)?;
            }
        }
        match found {
            0 => Ok(None),
            1 => edits.finish().map(Some),
            _ => Err(format!(
                "{self:?}: ambiguous production operation ({found})"
            )),
        }
    }
}
