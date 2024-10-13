use fxhash::FxHashMap;
use smol_str::SmolStr;

use super::pass0;
use crate::{
    ast::{
        Enum, Event, Expr, List, OnMessage, Proc, Project, References, Rrc, Sprite,
        Stmt, Struct, Var,
    },
    blocks::{BinOp, Block, UnOp},
};

struct V<'a> {
    references: &'a mut References,
    used_args: Option<&'a mut FxHashMap<SmolStr, bool>>,
}

struct S<'a> {
    vars: &'a FxHashMap<SmolStr, Var>,
    lists: &'a FxHashMap<SmolStr, List>,
    enums: &'a FxHashMap<SmolStr, Enum>,
    structs: &'a FxHashMap<SmolStr, Struct>,
    global_vars: Option<&'a FxHashMap<SmolStr, Var>>,
    global_lists: Option<&'a FxHashMap<SmolStr, List>>,
}

pub fn visit_project(project: &mut Project) {
    visit_sprite(&mut project.stage, None);
    for sprite in project.sprites.values_mut() {
        visit_sprite(sprite, Some(&project.stage));
    }
}

fn visit_sprite(sprite: &mut Sprite, stage: Option<&Sprite>) {
    let s = &mut S {
        vars: &sprite.vars,
        lists: &sprite.lists,
        enums: &sprite.enums,
        structs: &sprite.structs,
        global_vars: stage.map(|s| &s.vars),
        global_lists: stage.map(|s| &s.lists),
    };
    for event in &mut sprite.events {
        visit_event(event, s);
    }
    for on_message in sprite.on_messages.values_mut() {
        visit_on_message(on_message, s);
    }
    for proc in sprite.procs.values_mut() {
        visit_proc(proc, s);
    }
}

fn visit_proc(proc: &mut Proc, s: &mut S<'_>) {
    pass0::visit_proc(proc);
    visit_stmts(
        &mut proc.body,
        &mut V {
            references: &mut proc.references,
            used_args: Some(&mut proc.used_args),
        },
        s,
    );
}

fn visit_event(event: &mut Event, s: &mut S<'_>) {
    visit_stmts(
        &mut event.body,
        &mut V { references: &mut event.references, used_args: None },
        s,
    );
}

fn visit_on_message(on_message: &mut OnMessage, s: &mut S<'_>) {
    on_message.references.messages.insert(on_message.message.clone());
    visit_stmts(
        &mut on_message.body,
        &mut V { references: &mut on_message.references, used_args: None },
        s,
    );
}

fn visit_stmts(stmts: &mut Vec<Stmt>, v: &mut V<'_>, s: &mut S<'_>) {
    let mut i = 0;
    while i < stmts.len() {
        let mut stuff = vec![];
        match &stmts[i] {
            Stmt::SetVar { name, value, span, .. } => {
                if let Some(variable) = s.vars.get(name).or_else(|| {
                    s.global_vars.and_then(|global_vars| global_vars.get(name))
                }) {
                    if let Some(struct_) = variable
                        .type_
                        .struct_()
                        .and_then(|(name, _)| s.structs.get(name))
                    {
                        let value_brw = &*value.borrow();
                        let value_tuple = if let Expr::Name { name, span } = value_brw {
                            Some((name, span))
                        } else {
                            None
                        };
                        let value_variable =
                            value_tuple.and_then(|(value_variable, _)| {
                                s.vars.get(value_variable).or_else(|| {
                                    s.global_vars.and_then(|global_vars| {
                                        global_vars.get(value_variable)
                                    })
                                })
                            });
                        let value_variable_struct = value_variable
                            .and_then(|value_variable| value_variable.type_.struct_());

                        if let Some(value_variable_struct) = value_variable_struct {
                            let (value_name, value_span) = value_tuple.unwrap();
                            if *value_variable_struct.0 == struct_.name {
                                for (field, field_span) in &struct_.fields {
                                    stuff.push(Stmt::SetField {
                                        variable: name.clone(),
                                        variable_span: span.clone(),
                                        field: field.clone(),
                                        field_span: span.clone(),
                                        value: Expr::Accessor {
                                            symbol_name: value_name.clone(),
                                            symbol_span: value_span.clone(),
                                            property_name: field.clone(),
                                            property_span: field_span.clone(), // if this is being read, something is wrong
                                        }
                                        .into(),
                                    })
                                }
                            }
                        } else {
                            for (field, _) in &struct_.fields {
                                stuff.push(Stmt::SetField {
                                    variable: name.clone(),
                                    variable_span: span.clone(),
                                    field: field.clone(),
                                    field_span: span.clone(),
                                    value: value.clone(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if !stuff.is_empty() {
            stmts.remove(i);
        } else {
            i += 1;
        }
        for stmt in stuff.iter() {
            stmts.insert(i, stmt.clone());
            i += 1;
        }
    }
    for stmt in stmts {
        visit_stmt(stmt, v, s);
    }
}

fn visit_stmt(stmt: &mut Stmt, v: &mut V<'_>, s: &mut S<'_>) {
    match stmt {
        Stmt::Repeat { times, body } => {
            visit_expr(times, v, s);
            for stmt in body {
                visit_stmt(stmt, v, s);
            }
        }
        Stmt::Forever { body, span: _ } => {
            for stmt in body {
                visit_stmt(stmt, v, s);
            }
        }
        Stmt::Branch { cond, if_body, else_body } => {
            visit_expr(cond, v, s);
            for stmt in if_body {
                visit_stmt(stmt, v, s);
            }
            for stmt in else_body {
                visit_stmt(stmt, v, s);
            }
        }
        Stmt::Until { cond, body } => {
            visit_expr(cond, v, s);
            for stmt in body {
                visit_stmt(stmt, v, s);
            }
        }
        Stmt::SetVar { value, .. } => {
            // v.references.vars.insert(name.clone());
            // should set variable count as a reference?
            visit_expr(value, v, s);
        }
        Stmt::SetField { value, .. } => {
            visit_expr(value, v, s);
        }
        Stmt::ChangeVar { value, .. } => {
            // v.references.vars.insert(name.clone());
            // should change variable count as a reference?
            visit_expr(value, v, s);
        }
        Stmt::Show { name: _, span: _ } => {}
        Stmt::Hide { name: _, span: _ } => {}
        Stmt::ListAdd { name, span: _, value } => {
            v.references.lists.insert(name.clone());
            visit_expr(value, v, s);
        }
        Stmt::ListDelete { name, span: _, index } => {
            v.references.lists.insert(name.clone());
            visit_expr(index, v, s);
        }
        Stmt::ListDeleteAll { name, span: _ } => {
            v.references.lists.insert(name.clone());
        }
        Stmt::ListInsert { name, span: _, index, value } => {
            v.references.lists.insert(name.clone());
            visit_expr(index, v, s);
            visit_expr(value, v, s);
        }
        Stmt::ListSet { name, span: _, index, value } => {
            v.references.lists.insert(name.clone());
            visit_expr(index, v, s);
            visit_expr(value, v, s);
        }
        Stmt::ListChange { op: _, name, span: _, index, value } => {
            v.references.lists.insert(name.clone());
            visit_expr(index, v, s);
            visit_expr(value, v, s);
        }
        Stmt::Block { block, span: _, args } => {
            // reference the broadcast if this is a broadcast block.
            // if the broadcast argument is an expression, mark shadowed_message.
            // if shadowed_message is true,
            // then later down the line the broadcast block's shadowed input will be set to
            // an arbitrary broadcast, or a placeholder "message1" if no broadcasts exist in the project.
            if let Block::Broadcast | Block::BroadcastAndWait = block {
                if let Expr::Str(broadcast_name) = &mut *args[0].borrow_mut() {
                    v.references.messages.insert(broadcast_name.clone());
                }
            }

            for arg in args {
                visit_expr(arg, v, s);
            }
        }
        Stmt::ProcCall { name, span: _, args } => {
            v.references.procs.insert(name.clone());
            for arg in args {
                visit_expr(arg, v, s);
            }
        }
    }
}

fn visit_expr(expr: &mut Rrc<Expr>, v: &mut V<'_>, s: &mut S<'_>) {
    let mut replace: Option<Rrc<Expr>> = None;
    match &mut *expr.borrow_mut() {
        Expr::Int(_) => {}
        Expr::Float(_) => {}
        Expr::Str(_) => {}
        Expr::Accessor { symbol_name, property_name, .. } => {
            if s.enums.contains_key(symbol_name) {
                v.references
                    .enum_variants
                    .insert((symbol_name.clone(), property_name.clone()));
            } else if s.structs.contains_key(symbol_name) {
                v.references
                    .struct_fields
                    .insert((symbol_name.clone(), property_name.clone()));
            }
        }
        Expr::Name { name, .. } => {
            if s.vars.contains_key(name)
                || s.global_vars.is_some_and(|it| it.contains_key(name))
            {
                v.references.vars.insert(name.clone());
            } else {
                v.references.lists.insert(name.clone());
            }
        }
        Expr::Arg { name, .. } => {
            if let Some(used_args) = &mut v.used_args {
                if let Some(arg) = used_args.get_mut(name) {
                    *arg = true;
                }
            }
        }
        Expr::Repr { args, .. } => {
            for arg in args {
                visit_expr(arg, v, s);
            }
        }
        Expr::UnOp { op, val } => {
            visit_expr(val, v, s);
            match op {
                UnOp::Minus => match &mut *val.borrow_mut() {
                    Expr::Int(value) => {
                        *value = -*value;
                        replace = Some(val.clone());
                    }
                    Expr::Float(value) => {
                        *value = -*value;
                        replace = Some(val.clone());
                    }
                    Expr::BinOp { op: BinOp::Sub, lhs, rhs }
                        if lhs.borrow().is_zero() =>
                    {
                        replace = Some(rhs.clone());
                    }
                    _ => {
                        replace = Some(
                            BinOp::Sub.to_expr(Expr::Int(0).into(), val.clone()).into(),
                        );
                    }
                },
                UnOp::Not => {
                    if let Expr::UnOp { op: UnOp::Not, val } = &mut *val.borrow_mut() {
                        replace = Some(val.clone());
                    }
                }
                _ => {}
            }
        }
        Expr::BinOp { op, lhs, rhs } => {
            visit_expr(lhs, v, s);
            visit_expr(rhs, v, s);
            match op {
                BinOp::Of => {
                    if let Expr::Name { name, .. } = &*lhs.borrow() {
                        if s.lists.contains_key(name)
                            || s.global_lists.is_some_and(|it| it.contains_key(name))
                        {
                            v.references.lists.insert(name.clone());
                        }
                    }
                }
                BinOp::Add => match (&mut *lhs.borrow_mut(), &mut *rhs.borrow_mut()) {
                    (Expr::Int(lval), Expr::Int(rval)) => {
                        *lval += *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Int(lval), Expr::Float(rval)) => {
                        *rval += *lval as f64;
                        replace = Some(rhs.clone());
                    }
                    (Expr::Float(lval), Expr::Float(rval)) => {
                        *lval += *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Float(lval), Expr::Int(rval)) => {
                        *lval += *rval as f64;
                        replace = Some(lhs.clone());
                    }
                    _ => {}
                },
                BinOp::Sub => match (&mut *lhs.borrow_mut(), &mut *rhs.borrow_mut()) {
                    (Expr::Int(lval), Expr::Int(rval)) => {
                        *lval -= *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Int(lval), Expr::Float(rval)) => {
                        *rval = *lval as f64 - *rval;
                        replace = Some(rhs.clone());
                    }
                    (Expr::Float(lval), Expr::Float(rval)) => {
                        *lval -= *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Float(lval), Expr::Int(rval)) => {
                        *lval -= *rval as f64;
                        replace = Some(lhs.clone());
                    }
                    _ => {}
                },
                BinOp::Mul => match (&mut *lhs.borrow_mut(), &mut *rhs.borrow_mut()) {
                    (Expr::Int(lval), Expr::Int(rval)) => {
                        *lval *= *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Int(lval), Expr::Float(rval)) => {
                        *rval *= *lval as f64;
                        replace = Some(rhs.clone());
                    }
                    (Expr::Float(lval), Expr::Float(rval)) => {
                        *lval *= *rval;
                        replace = Some(lhs.clone());
                    }
                    (Expr::Float(lval), Expr::Int(rval)) => {
                        *lval *= *rval as f64;
                        replace = Some(lhs.clone());
                    }
                    _ => {}
                },
                BinOp::Le => {
                    replace = Some(
                        UnOp::Not
                            .to_expr(BinOp::Lt.to_expr(rhs.clone(), lhs.clone()).into())
                            .into(),
                    )
                }
                BinOp::Ge => {
                    replace = Some(
                        UnOp::Not
                            .to_expr(BinOp::Gt.to_expr(rhs.clone(), lhs.clone()).into())
                            .into(),
                    )
                }
                BinOp::Ne => {
                    replace = Some(
                        UnOp::Not
                            .to_expr(BinOp::Eq.to_expr(lhs.clone(), rhs.clone()).into())
                            .into(),
                    )
                }
                BinOp::FloorDiv => {
                    replace = Some(
                        UnOp::Floor
                            .to_expr(
                                BinOp::Div.to_expr(lhs.clone(), rhs.clone()).into(),
                            )
                            .into(),
                    )
                }
                _ => {}
            }
        }
    }
    if let Some(replace) = replace {
        *expr = replace;
    }
}
