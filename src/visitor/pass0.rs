use fxhash::FxHashMap;
use smol_str::SmolStr;

use crate::ast::*;

struct V<'a> {
    locals: Option<&'a mut FxHashMap<SmolStr, Var>>,
    vars: &'a mut FxHashMap<SmolStr, Var>,
    global_vars: Option<&'a mut FxHashMap<SmolStr, Var>>,
}

pub fn visit_project(project: &mut Project) {
    visit_sprite(&mut project.stage, None);
    for sprite in project.sprites.values_mut() {
        visit_sprite(sprite, Some(&mut project.stage));
    }
}

fn visit_sprite(sprite: &mut Sprite, mut stage: Option<&mut Sprite>) {
    for proc in sprite.procs.values_mut() {
        visit_stmts(
            &mut proc.body,
            &mut V {
                locals: Some(&mut proc.locals),
                vars: &mut sprite.vars,
                global_vars: stage.as_mut().map(|stage| &mut stage.vars),
            },
        );
    }
    for event in &mut sprite.events {
        visit_stmts(
            &mut event.body,
            &mut V { locals: None, vars: &mut sprite.vars, global_vars: stage.as_mut().map(|stage| &mut stage.vars) },
        );
    }
}

fn visit_stmts(stmts: &mut Vec<Stmt>, v: &mut V) {
    for stmt in stmts {
        visit_stmt(stmt, v);
    }
}

fn visit_stmt(stmt: &mut Stmt, v: &mut V) {
    match stmt {
        Stmt::Repeat { body, .. } => {
            visit_stmts(body, v);
        }
        Stmt::Forever { body, .. } => {
            visit_stmts(body, v);
        }
        Stmt::Branch { if_body, else_body, .. } => {
            visit_stmts(if_body, v);
            visit_stmts(else_body, v);
        }
        Stmt::Until { body, .. } => {
            visit_stmts(body, v);
        }
        Stmt::SetVar { name, type_, is_local, .. } => {
            let span = name.span();
            let name = name.basename();
            if let Some(locals) = &mut v.locals {
                if *is_local {
                    locals.insert(name.clone(), Var { name: name.clone(), span: span.clone(), type_: type_.clone() });
                }
            }
            if !(v.global_vars.as_ref().is_some_and(|global_vars| global_vars.contains_key(name)) || v.vars.contains_key(name))
            {
                if *is_local {
                    if let Some(locals) = &mut v.locals {
                        locals.insert(name.clone(), Var { name: name.clone(), span: span.clone(), type_: type_.clone() });
                    }
                } else {
                    v.vars.insert(name.clone(), Var { name: name.clone(), span: span.clone(), type_: type_.clone() });
                }
            }
        }
        _ => {}
    }
}
