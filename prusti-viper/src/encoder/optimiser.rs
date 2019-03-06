// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::mem;
use std::collections::HashMap;
use encoder::vir;


/// Optimisations currently done:
///
/// 1.  Replace all `old(...)` inside `forall` with
///     `let tmp == (old(..)) in forall`.
pub fn rewrite(cfg: vir::CfgMethod) -> vir::CfgMethod {
    let mut optimiser = Optimiser::new();
    optimiser.replace_cfg(cfg)
}

struct Optimiser {
}

impl Optimiser {
    fn new() -> Self {
        Self {
        }
    }

    fn replace_cfg(&mut self, mut cfg: vir::CfgMethod) -> vir::CfgMethod {
        let mut sentinel_stmt = vir::Stmt::Comment(String::from("moved out stmt"));
        for block in &mut cfg.basic_blocks {
            for stmt in &mut block.stmts {
                mem::swap(&mut sentinel_stmt, stmt);
                sentinel_stmt = self.replace_stmt(sentinel_stmt);
                mem::swap(&mut sentinel_stmt, stmt);
            }
        }
        cfg
    }

    fn replace_stmt(&mut self, stmt: vir::Stmt) -> vir::Stmt {
        use self::vir::StmtFolder;
        self.fold(stmt)
    }

    fn replace_expr(&mut self, expr: vir::Expr) -> vir::Expr {
        use self::vir::ExprFolder;
        self.fold(expr)
    }
}


impl vir::StmtFolder for Optimiser {

    fn fold_assert(&mut self, e: vir::Expr, p: vir::Position) -> vir::Stmt {
        vir::Stmt::Assert(self.replace_expr(e), p)
    }

}

impl vir::ExprFolder for Optimiser {

    fn fold_forall(
        &mut self,
        variables: Vec<vir::LocalVar>,
        triggers: Vec<vir::Trigger>,
        body: Box<vir::Expr>,
        pos: vir::Position
    ) -> vir::Expr {

        debug!("original body: {}", body);
        let mut replacer = OldPlaceReplacer::new();
        let mut replaced_body = replacer.fold_boxed(body);
        debug!("replaced body: {}", replaced_body);
        let mut forall = vir::Expr::ForAll(variables, triggers, replaced_body, pos.clone());

        if replacer.counter > 0 {
            for (expr, variable) in replacer.map {
                forall = vir::Expr::LetExpr(variable, box expr, box forall, pos.clone());
            }
            debug!("replaced quantifier: {}", forall);
        }

        forall
    }

}

struct OldPlaceReplacer {
    counter: u32,
    map: HashMap<vir::Expr, vir::LocalVar>,
}

impl OldPlaceReplacer {
    fn new() -> Self {
        Self {
            counter: 0,
            map: HashMap::new(),
        }
    }
    fn construct_fresh_local(&mut self, ty: &vir::Type) -> vir::LocalVar {
        let name = format!("_LET_{}", self.counter);
        self.counter += 1;
        vir::LocalVar {
            name: name,
            typ: ty.clone(),
        }
    }
}

impl vir::ExprFolder for OldPlaceReplacer {

    fn fold_labelled_old(
        &mut self,
        label: String,
        expr: Box<vir::Expr>,
        pos: vir::Position
    ) -> vir::Expr {
        if expr.is_place() {
            if let Some(local) = self.map.get(&expr) {
                vir::Expr::Local(local.clone(), pos)
            } else {
                let ty = expr.get_type();
                let local = self.construct_fresh_local(ty);
                self.map.insert(*expr, local.clone());
                vir::Expr::Local(local, pos)
            }
        } else {
            vir::Expr::LabelledOld(label, expr, pos)
        }
    }
}
