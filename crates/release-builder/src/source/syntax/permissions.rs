//! Native TUI presentation of Termux's existing three permission choices.
//! Decisions and approval enforcement stay in upstream; these edits only bind
//! its local UI to the sandbox profile supported by the Termux runtime.
use super::*;
use syn::{BinOp, Pat, Stmt, UnOp};

fn pattern_strings(pattern: &Pat) -> Option<Vec<String>> {
    match pattern {
        Pat::Lit(value) => literal(&Expr::Lit(value.clone())).map(|value| vec![value]),
        Pat::Or(or) => or
            .cases
            .iter()
            .map(pattern_strings)
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.into_iter().flatten().collect()),
        _ => None,
    }
}

fn matches_args(mac: &syn::ExprMacro) -> Result<(Expr, Pat)> {
    let parser = |input: syn::parse::ParseStream<'_>| {
        let expr = input.parse::<Expr>()?;
        input.parse::<syn::Token![,]>()?;
        let pat = Pat::parse_multi_with_leading_vert(input)?;
        Ok((expr, pat))
    };
    parser
        .parse2(mac.mac.tokens.clone())
        .map_err(|e| format!("permission match syntax: {e}"))
}

pub(super) fn adapt(
    operation: Operation,
    nodes: &Nodes<'_>,
    edits: &mut Edits<'_>,
) -> Result<usize> {
    let mut found = 0;
    match operation {
        Operation::AutoReviewMode => {
            for expr in &nodes.exprs {
                let Expr::Struct(mode) = expr else {
                    continue;
                };
                if mode
                    .path
                    .segments
                    .last()
                    .is_none_or(|s| s.ident != "AutoReviewMode")
                {
                    continue;
                }
                let Some(field) = mode.fields.iter().find(
                    |f| matches!(&f.member, Member::Named(id) if id == "active_permission_profile"),
                ) else {
                    return Err("auto-review UI: active profile missing".into());
                };
                if !tokens(&field.expr).contains(WORKSPACE) {
                    return Err("auto-review UI: sandbox contract changed".into());
                }
                edits.set(
                    field.expr.span(),
                    edits
                        .source(&field.expr)
                        .replace(WORKSPACE, &format!("codex_protocol::models::{FULL_ACCESS}")),
                );
                found += 1;
            }
        }
        Operation::ResumePermissions => {
            for expr in &nodes.exprs {
                let Expr::If(branch) = expr else {
                    continue;
                };
                let Expr::Binary(condition) = &*branch.cond else {
                    continue;
                };
                let Expr::Unary(right) = &*condition.right else {
                    continue;
                };
                let Expr::Field(field) = &*right.expr else {
                    continue;
                };
                if !matches!(condition.op, BinOp::Or(_))
                    || !matches!(right.op, UnOp::Not(_))
                    || !matches!(&field.member, Member::Named(id) if id == "profile")
                    || !tokens(&condition.left).contains("ThreadParamsMode :: Remote")
                {
                    continue;
                }
                if !tokens(&branch.then_branch).contains("remove_permission_config_overrides") {
                    return Err("local resume: permission override consumer changed".into());
                }
                edits.set(condition.span(), edits.source(&condition.left));
                found += 1;
            }
        }
        Operation::PermissionShortcut => {
            for expr in &nodes.exprs {
                let Expr::Unary(negated) = expr else {
                    continue;
                };
                let Expr::Macro(mac) = &*negated.expr else {
                    continue;
                };
                if !matches!(negated.op, UnOp::Not(_)) || !mac.mac.path.is_ident("matches") {
                    continue;
                }
                if !tokens(mac).contains("\"read-only\"") || !tokens(mac).contains("\"auto\"") {
                    continue;
                }
                let (value, pattern) = matches_args(mac)?;
                if pattern_strings(&pattern).as_deref()
                    != Some(&["read-only".into(), "auto".into()])
                {
                    continue;
                }
                edits.set(
                    negated.span(),
                    format!("{} != \"auto\"", edits.source(&value)),
                );
                found += 1;
            }
        }
        Operation::PermissionMatch => {
            for expr in &nodes.exprs {
                let Expr::Match(matched) = expr else {
                    continue;
                };
                let Some(auto) = matched
                    .arms
                    .iter()
                    .find(|arm| pattern_strings(&arm.pat).as_deref() == Some(&["auto".into()]))
                else {
                    continue;
                };
                let Some(full) = matched.arms.iter().find(|arm| {
                    pattern_strings(&arm.pat).as_deref() == Some(&["full-access".into()])
                }) else {
                    continue;
                };
                let Expr::Macro(mac) = &*full.body else {
                    continue;
                };
                if !mac.mac.path.is_ident("matches")
                    || !tokens(mac).contains("PermissionProfile :: Disabled")
                {
                    continue;
                }
                if !tokens(&auto.body).contains("can_write_local_path_with_cwd") {
                    return Err("auto preset UI: sandbox comparison changed".into());
                }
                edits.set(
                    auto.body.span(),
                    format!("{{ {} }}", edits.source(&full.body)),
                );
                found += 1;
            }
        }
        Operation::PermissionSelection => {
            for expr in &nodes.exprs {
                let Expr::Match(matched) = expr else {
                    continue;
                };
                for arm in &matched.arms {
                    if pattern_strings(&arm.pat).as_deref()
                        != Some(&[":ask-for-approval".into(), ":approve-for-me".into()])
                    {
                        continue;
                    }
                    if leaf(&arm.body).as_deref() != Some(WORKSPACE) {
                        return Err("permission selection: profile mapping changed".into());
                    }
                    edits.set(
                        arm.body.span(),
                        format!("codex_protocol::models::{FULL_ACCESS}"),
                    );
                    found += 1;
                }
            }
        }
        Operation::PermissionDescription => {
            for expr in &nodes.exprs {
                let Expr::Match(matched) = expr else {
                    continue;
                };
                let Some(arm) = matched
                    .arms
                    .iter()
                    .find(|arm| pattern_strings(&arm.pat).as_deref() == Some(&["auto".into()]))
                else {
                    continue;
                };
                let Expr::Block(block) = &*arm.body else {
                    continue;
                };
                let [Stmt::Expr(value, None)] = block.block.stmts.as_slice() else {
                    continue;
                };
                if !literal(value)
                    .is_some_and(|text| text.contains("workspace") && text.contains("approval"))
                {
                    continue;
                }
                edits.set(value.span(), format!("{DESCRIPTION:?}"));
                found += 1;
            }
        }
        Operation::PermissionMenu => {
            for expr in &nodes.exprs {
                let Expr::MethodCall(call) = expr else {
                    continue;
                };
                if !call
                    .args
                    .iter()
                    .any(|arg| literal(arg).as_deref() == Some(":read-only"))
                {
                    continue;
                }
                let bindings: Vec<_> = call
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        Expr::Path(path) if path.path.segments.len() == 1 => {
                            Some(path.path.segments[0].ident.to_string())
                        }
                        _ => None,
                    })
                    .collect();
                if bindings.len() != 1 {
                    return Err("permissions popup: read-only preset binding changed".into());
                }
                let binding = &bindings[0];
                let (_, scope) = nodes
                    .functions
                    .iter()
                    .find(|(_, span)| within(call.span(), *span))
                    .ok_or("permissions popup scope missing")?;
                let statement = nodes
                    .statements
                    .iter()
                    .filter(|stmt| within(call.span(), stmt.span()))
                    .min_by_key(|stmt| stmt.span().byte_range().len())
                    .ok_or("permissions popup statement missing")?;
                if !matches!(statement, Stmt::Expr(Expr::MethodCall(push), _) if push.method == "push")
                {
                    return Err("permissions popup: item insertion contract changed".into());
                }
                let declarations: Vec<_> = nodes.statements.iter().filter_map(|stmt| match stmt {
                Stmt::Local(local) if within(local.span(), *scope) && matches!(&local.pat,
                    Pat::TupleStruct(some) if some.path.is_ident("Some") && some.elems.len() == 1 && matches!(&some.elems[0], Pat::Ident(id) if &id.ident.to_string() == binding)) => Some(local), _ => None,
            }).collect();
                let other_consumers = nodes.exprs.iter().any(|expr| {
                    within(expr.span(), *scope)
                        && leaf(expr).as_ref() == Some(binding)
                        && !within(expr.span(), statement.span())
                });
                if declarations.len() != 1 || other_consumers {
                    return Err(
                        "permissions popup: read-only preset has additional consumers".into(),
                    );
                }
                edits.remove_statement(statement.span());
                edits.remove_statement(declarations[0].span());
                let choices: Vec<_> = nodes
                    .exprs
                    .iter()
                    .filter(|expr| {
                        within(expr.span(), *scope)
                            && literal(expr).as_deref() == Some(":workspace")
                    })
                    .collect();
                if choices.len() != 2 {
                    return Err("permissions popup: Ask and Auto choices changed".into());
                }
                for choice in choices {
                    edits.set(
                        choice.span(),
                        format!("codex_protocol::models::{FULL_ACCESS}"),
                    );
                }
                found += 1;
            }
        }
        Operation::AutoReviewDescription => {
            for item in &nodes.constants {
                if literal(&item.expr).as_deref()
                    != Some("Only ask for actions detected as potentially unsafe")
                {
                    continue;
                }
                edits.set(
                    item.expr.span(),
                    format!("{:?}", super::super::REVIEW_DESCRIPTION),
                );
                found += 1;
            }
        }
        _ => return Err("unexpected permission presentation operation".into()),
    }
    Ok(found)
}
