use std::io::{self, Seek, Write};

use logos::Span;

use super::{
    node::Node,
    node_id::NodeID,
    sb3::{qualify_struct_var_name, QualifiedName, Sb3, D, S},
};
use crate::{
    ast::*,
    blocks::{BinOp, Repr, UnOp},
    diagnostic::DiagnosticKind,
    misc::Rrc,
};

impl<T> Sb3<T>
where T: Write + Seek
{
    pub fn arg(&mut self, s: S, d: D, this_id: NodeID, parent_id: NodeID, name: &Name) -> io::Result<()> {
        let basename = name.basename();
        let Some(proc) = s.proc else {
            d.report(DiagnosticKind::UnrecognizedArgument(basename.clone()), &name.span());
            return Ok(());
        };
        if !proc.args.iter().any(|arg| &arg.name == basename) {
            d.report(DiagnosticKind::UnrecognizedArgument(basename.clone()), &name.span());
            return Ok(());
        }
        let qualified_name = match name.fieldname() {
            Some(fieldname) => qualify_struct_var_name(fieldname, basename),
            None => basename.clone(),
        };
        self.begin_node(Node::new("argument_reporter_string_number", this_id).parent_id(parent_id))?;
        self.single_field("VALUE", &qualified_name)?;
        self.end_obj() // node
    }

    pub fn repr(
        &mut self,
        s: S,
        d: D,
        this_id: NodeID,
        parent_id: NodeID,
        repr: &Repr,
        span: &Span,
        args: &Vec<Rrc<Expr>>,
    ) -> io::Result<()> {
        if args.len() != repr.args().len() {
            todo!()
        }
        let arg_ids = (&mut self.id).take(args.len()).collect::<Vec<_>>();
        self.begin_node(Node::new(repr.opcode(), this_id).parent_id(parent_id))?;
        let menu_id = repr.menu().map(|_| self.id.new_id());
        let mut menu_value = None;
        let mut menu_is_default = menu_id.is_some();
        self.begin_inputs()?;
        for ((&arg_name, arg_expr), arg_id) in repr.args().iter().zip(args).zip(arg_ids) {
            if repr.menu().is_some_and(|menu| menu.input == arg_name) {
                if let Expr::Value { value, .. } = &*arg_expr.borrow() {
                    menu_value = Some(value);
                    continue;
                } else {
                    menu_is_default = false;
                    todo!()
                }
            } else {
                todo!()
            }
        }
        todo!()
    }

    pub fn un_op(
        &mut self,
        s: S,
        d: D,
        this_id: NodeID,
        parent_id: NodeID,
        op: &UnOp,
        _span: &Span,
        opr: &Rrc<Expr>,
    ) -> io::Result<()> {
        let opr_id = self.id.new_id();
        self.begin_node(Node::new(op.opcode(), this_id).parent_id(parent_id))?;
        self.begin_inputs()?;
        self.input(s, d, op.input(), &opr.borrow(), opr_id)?;
        self.end_obj()?; // inputs
        if let Some(fields) = op.fields() {
            write!(self, r#","fields":{fields}"#)?;
        }
        self.end_obj()?; // node
        self.expr(s, d, &opr.borrow(), opr_id, this_id)
    }

    pub fn bin_op(
        &mut self,
        s: S,
        d: D,
        this_id: NodeID,
        parent_id: NodeID,
        op: &BinOp,
        _span: &Span,
        lhs: &Rrc<Expr>,
        rhs: &Rrc<Expr>,
    ) -> io::Result<()> {
        if let BinOp::Of = op {
            if let Expr::Name(name) = &*lhs.borrow() {
                if let Some(QualifiedName::List(qualified_name, _)) = s.qualify_name(d, name) {
                    return self.list_index(s, d, this_id, parent_id, &qualified_name, rhs);
                }
            }
        }
        let lhs_id = self.id.new_id();
        let rhs_id = self.id.new_id();
        self.begin_node(Node::new(op.opcode(), this_id).parent_id(parent_id))?;
        self.begin_inputs()?;
        self.input(s, d, op.lhs(), &lhs.borrow(), lhs_id)?;
        self.input(s, d, op.rhs(), &rhs.borrow(), rhs_id)?;
        self.end_obj()?; // inputs
        self.end_obj()?; // node
        self.expr(s, d, &lhs.borrow(), lhs_id, this_id)?;
        self.expr(s, d, &rhs.borrow(), rhs_id, this_id)
    }

    pub fn list_index(
        &mut self,
        s: S,
        d: D,
        this_id: NodeID,
        parent_id: NodeID,
        name: &str,
        index: &Rrc<Expr>,
    ) -> io::Result<()> {
        let index_id = self.id.new_id();
        self.begin_node(Node::new("data_itemoflist", this_id).parent_id(parent_id))?;
        self.begin_inputs()?;
        self.input(s, d, "INDEX", &index.borrow(), index_id)?;
        self.end_obj()?; // inputs
        self.single_field_id("LIST", name)?;
        self.end_obj()?; // node
        self.expr(s, d, &index.borrow(), index_id, this_id)
    }
}
