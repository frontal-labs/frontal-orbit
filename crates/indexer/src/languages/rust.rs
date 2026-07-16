//! Rust-specific indexer using syn.

use crate::languages::LanguageIndexer;
use crate::{
    Export, FileId, Import, IndexedFile, IndexerResult, Language, RefKind, Span, Symbol, SymbolId,
    SymbolKind, SymbolRef, Visibility,
};
use std::collections::HashMap;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMethodCall, Field, FnArg, GenericParam, ImplItemFn, ItemConst,
    ItemEnum, ItemFn, ItemImpl, ItemMacro, ItemMod, ItemStatic, ItemStruct, ItemTrait, ItemType,
    ItemUse, Lit, Meta, Pat, StaticMutability, TraitItemFn, Variant,
};

/// Rust indexer using syn for accurate AST parsing.
pub struct RustIndexer {
    file_id_counter: std::sync::atomic::AtomicU64,
}

impl RustIndexer {
    pub fn new() -> Self {
        Self {
            file_id_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_file_id(&self) -> FileId {
        FileId(
            self.file_id_counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        )
    }
}

impl Default for RustIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageIndexer for RustIndexer {
    fn index_file(&self, file_path: &Path, content: &str) -> IndexerResult<IndexedFile> {
        let file_id = self.next_file_id();
        let syntax_tree = syn::parse_file(content)?;

        let mut visitor = RustVisitor::new(file_id);
        visitor.visit_file(&syntax_tree);

        let mut indexed_file = IndexedFile {
            id: file_id,
            path: file_path.to_path_buf(),
            language: Language::Rust,
            hash: {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                format!("{:x}", hasher.finish())
            },
            symbols: visitor.symbols,
            references: visitor.references,
            imports: visitor.imports,
            exports: visitor.exports,
            module_path: visitor.module_path,
            last_modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Resolve references to symbol IDs
        resolve_references(&mut indexed_file);

        Ok(indexed_file)
    }

    fn language(&self) -> Language {
        Language::Rust
    }

    fn extensions(&self) -> &[&str] {
        static EXTS: &[&str] = &["rs"];
        EXTS
    }
}

/// Visitor for extracting symbols and references from Rust AST.
struct RustVisitor {
    file_id: FileId,
    symbols: Vec<Symbol>,
    references: Vec<SymbolRef>,
    imports: Vec<Import>,
    exports: Vec<Export>,
    module_path: Vec<String>,
    current_item_path: Vec<String>,
    in_test_module: bool,
    in_impl_block: Option<String>,
}

#[allow(clippy::unused_self)]
impl RustVisitor {
    fn new(file_id: FileId) -> Self {
        Self {
            file_id,
            symbols: Vec::new(),
            references: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            module_path: vec!["crate".to_string()],
            current_item_path: Vec::new(),
            in_test_module: false,
            in_impl_block: None,
        }
    }

    fn add_symbol(&mut self, mut symbol: Symbol) {
        symbol.id.file_id = self.file_id;
        if !self.current_item_path.is_empty() {
            symbol.id.parent = Some(Box::new(SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Module,
                name: self.current_item_path.join("::"),
                parent: None,
            }));
        }
        self.symbols.push(symbol);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn make_span(&self, node: &impl Spanned) -> Span {
        let start = node.span().start();
        let end = node.span().end();
        Span {
            file_id: self.file_id,
            start_line: start.line as u32,
            start_column: start.column as u32,
            end_line: end.line as u32,
            end_column: end.column as u32,
            start_offset: 0, // Would need SourceFile to compute
            end_offset: 0,
        }
    }

    fn visibility_from(&self, vis: &syn::Visibility) -> Visibility {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(vis_restricted) => {
                if vis_restricted.in_token.is_none() {
                    Visibility::PubCrate
                } else {
                    Visibility::Private
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }

    fn extract_doc_comment(&self, attrs: &[Attribute]) -> Option<String> {
        attrs
            .iter()
            .filter_map(|attr| {
                if attr.path().is_ident("doc") {
                    if let Meta::NameValue(meta_nv) = &attr.meta {
                        if let Expr::Lit(expr_lit) = &meta_nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into()
    }

    fn extract_attributes(&self, attrs: &[Attribute]) -> Vec<String> {
        attrs
            .iter()
            .map(|attr| {
                attr.path()
                    .get_ident()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
            })
            .filter(|s| !s.is_empty() && *s != "doc")
            .collect()
    }
}

#[allow(clippy::unused_self)]
impl<'ast> Visit<'ast> for RustVisitor {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let mod_name = node.ident.to_string();
        self.module_path.push(mod_name.clone());
        self.current_item_path.push(mod_name.clone());

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Module,
                name: mod_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Module,
            name: mod_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);

        if let Some((_, items)) = &node.content {
            for item in items {
                visit::visit_item(self, item);
            }
        }

        self.module_path.pop();
        self.current_item_path.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let fn_name = node.sig.ident.to_string();
        self.current_item_path.push(fn_name.clone());

        let is_async = node.sig.asyncness.is_some();
        let is_unsafe = node.sig.unsafety.is_some();
        let is_const = node.sig.constness.is_some();

        let mut generics = Vec::new();
        for param in &node.sig.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let mut signature_parts = Vec::new();
        for input in &node.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    signature_parts.push(format!(
                        "{}: {}",
                        pat_ident.ident,
                        quote::quote!(#pat_type.ty)
                    ));
                }
            }
        }
        let signature = format!(
            "fn {}({}) -> {}",
            fn_name,
            signature_parts.join(", "),
            quote::quote!(#node.sig.output)
        );

        let kind = if is_async {
            SymbolKind::AsyncFunction
        } else if self.in_impl_block.is_some() {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };

        let _is_test = self.in_test_module
            || fn_name.starts_with("test_")
            || node.attrs.iter().any(|a| a.path().is_ident("test"));

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind,
                name: fn_name.clone(),
                parent: None,
            },
            kind,
            name: fn_name,
            signature: Some(signature),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async,
            is_unsafe,
            is_const,
        };
        self.add_symbol(symbol);

        // Visit function body for calls
        visit::visit_block(self, &node.block);

        self.current_item_path.pop();
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let struct_name = node.ident.to_string();
        self.current_item_path.push(struct_name.clone());

        let mut generics = Vec::new();
        for param in &node.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Struct,
                name: struct_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Struct,
            name: struct_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);

        // Visit fields
        for field in &node.fields {
            visit::visit_field(self, field);
        }

        self.current_item_path.pop();
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        let enum_name = node.ident.to_string();
        self.current_item_path.push(enum_name.clone());

        let mut generics = Vec::new();
        for param in &node.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Enum,
                name: enum_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Enum,
            name: enum_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);

        // Visit variants
        for variant in &node.variants {
            visit::visit_variant(self, variant);
        }

        self.current_item_path.pop();
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let trait_name = node.ident.to_string();
        self.current_item_path.push(trait_name.clone());

        let mut generics = Vec::new();
        for param in &node.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Trait,
                name: trait_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Trait,
            name: trait_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);

        // Visit trait items
        for item in &node.items {
            visit::visit_trait_item(self, item);
        }

        self.current_item_path.pop();
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let impl_type = quote::quote!(#node.self_ty).to_string();
        self.in_impl_block = Some(impl_type.clone());
        self.current_item_path.push(format!("impl {impl_type}"));

        // Visit impl items
        for item in &node.items {
            visit::visit_impl_item(self, item);
        }

        self.current_item_path.pop();
        self.in_impl_block = None;
    }

    fn visit_item_const(&mut self, node: &'ast ItemConst) {
        let const_name = node.ident.to_string();

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Const,
                name: const_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Const,
            name: const_name,
            signature: Some(format!(
                "const {}: {} = {}",
                node.ident,
                quote::quote!(#node.ty),
                quote::quote!(#node.expr)
            )),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: true,
        };
        self.add_symbol(symbol);

        visit::visit_expr(self, &node.expr);
    }

    fn visit_item_static(&mut self, node: &'ast ItemStatic) {
        let static_name = node.ident.to_string();

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Static,
                name: static_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Static,
            name: static_name,
            signature: Some(format!(
                "static {}: {} = {}",
                node.ident,
                quote::quote!(#node.ty),
                quote::quote!(#node.expr)
            )),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: matches!(node.mutability, StaticMutability::Mut(_)),
            is_const: false,
        };
        self.add_symbol(symbol);

        visit::visit_expr(self, &node.expr);
    }

    fn visit_item_type(&mut self, node: &'ast ItemType) {
        let type_name = node.ident.to_string();

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::TypeAlias,
                name: type_name.clone(),
                parent: None,
            },
            kind: SymbolKind::TypeAlias,
            name: type_name,
            signature: Some(format!("type {} = {}", node.ident, quote::quote!(#node.ty))),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);
    }

    fn visit_item_macro(&mut self, node: &'ast ItemMacro) {
        let macro_name = node
            .mac
            .path
            .get_ident()
            .map(std::string::ToString::to_string)
            .unwrap_or_default();

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Macro,
                name: macro_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Macro,
            name: macro_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: Visibility::Private,
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        let use_path = quote::quote!(#node.tree).to_string();
        let span = self.make_span(node);

        let import = Import {
            path: use_path.clone(),
            items: vec![],
            span,
            is_glob: use_path.contains('*'),
        };
        self.imports.push(import);

        // Also track as export if pub use
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let export = Export {
                name: use_path,
                symbol_id: None,
                span,
                reexport: Some("pub use".to_string()),
            };
            self.exports.push(export);
        }
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        // Record a function call
        if let Expr::Path(path) = &*node.func {
            if let Some(ident) = path.path.get_ident() {
                let ref_ = SymbolRef {
                    symbol_id: SymbolId {
                        file_id: self.file_id,
                        kind: SymbolKind::Function,
                        name: ident.to_string(),
                        parent: None,
                    },
                    kind: RefKind::Call,
                    span: self.make_span(node),
                    context: self.current_item_path.last().cloned(),
                };
                self.references.push(ref_);
            }
        } else if let Expr::MethodCall(method_call) = &*node.func {
            let ref_ = SymbolRef {
                symbol_id: SymbolId {
                    file_id: self.file_id,
                    kind: SymbolKind::Method,
                    name: method_call.method.to_string(),
                    parent: None,
                },
                kind: RefKind::MethodCall,
                span: self.make_span(node),
                context: self.current_item_path.last().cloned(),
            };
            self.references.push(ref_);
        }

        // Visit arguments
        for arg in &node.args {
            visit::visit_expr(self, arg);
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let ref_ = SymbolRef {
            symbol_id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Method,
                name: node.method.to_string(),
                parent: None,
            },
            kind: RefKind::MethodCall,
            span: self.make_span(node),
            context: self.current_item_path.last().cloned(),
        };
        self.references.push(ref_);

        for arg in &node.args {
            visit::visit_expr(self, arg);
        }
    }

    fn visit_field(&mut self, node: &'ast Field) {
        if let Some(ident) = &node.ident {
            let field_name = ident.to_string();

            let symbol = Symbol {
                id: SymbolId {
                    file_id: self.file_id,
                    kind: SymbolKind::Field,
                    name: field_name.clone(),
                    parent: None,
                },
                kind: SymbolKind::Field,
                name: field_name,
                signature: Some(quote::quote!(#node.ty).to_string()),
                doc_comment: self.extract_doc_comment(&node.attrs),
                span: self.make_span(node),
                visibility: self.visibility_from(&node.vis),
                attributes: self.extract_attributes(&node.attrs),
                generics: vec![],
                is_async: false,
                is_unsafe: false,
                is_const: false,
            };
            self.add_symbol(symbol);
        }
        visit::visit_type(self, &node.ty);
    }

    fn visit_variant(&mut self, node: &'ast Variant) {
        let variant_name = node.ident.to_string();

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Variant,
                name: variant_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Variant,
            name: variant_name,
            signature: None,
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: Visibility::Private,
            attributes: self.extract_attributes(&node.attrs),
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        self.add_symbol(symbol);

        for field in &node.fields {
            visit::visit_field(self, field);
        }
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        let fn_name = node.sig.ident.to_string();
        let is_async = node.sig.asyncness.is_some();
        let is_unsafe = node.sig.unsafety.is_some();

        let mut generics = Vec::new();
        for param in &node.sig.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: if is_async {
                    SymbolKind::AsyncFunction
                } else {
                    SymbolKind::Function
                },
                name: fn_name.clone(),
                parent: None,
            },
            kind: if is_async {
                SymbolKind::AsyncFunction
            } else {
                SymbolKind::Function
            },
            name: fn_name,
            signature: Some(format!(
                "fn {}(...) -> {}",
                node.sig.ident,
                quote::quote!(#node.sig.output)
            )),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: Visibility::Public, // Trait items are public
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async,
            is_unsafe,
            is_const: false,
        };
        self.add_symbol(symbol);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let fn_name = node.sig.ident.to_string();
        let is_async = node.sig.asyncness.is_some();
        let is_unsafe = node.sig.unsafety.is_some();
        let is_const = node.sig.constness.is_some();

        let mut generics = Vec::new();
        for param in &node.sig.generics.params {
            if let GenericParam::Type(type_param) = param {
                generics.push(type_param.ident.to_string());
            }
        }

        let symbol = Symbol {
            id: SymbolId {
                file_id: self.file_id,
                kind: SymbolKind::Method,
                name: fn_name.clone(),
                parent: None,
            },
            kind: SymbolKind::Method,
            name: fn_name,
            signature: Some(format!(
                "fn {}(...) -> {}",
                node.sig.ident,
                quote::quote!(#node.sig.output)
            )),
            doc_comment: self.extract_doc_comment(&node.attrs),
            span: self.make_span(node),
            visibility: self.visibility_from(&node.vis),
            attributes: self.extract_attributes(&node.attrs),
            generics,
            is_async,
            is_unsafe,
            is_const,
        };
        self.add_symbol(symbol);

        visit::visit_block(self, &node.block);
    }

    fn visit_attribute(&mut self, node: &'ast Attribute) {
        // Check for test modules
        if node.path().is_ident("cfg") {
            // Check if it's #[cfg(test)]
            let attr_str = quote::quote!(#node).to_string();
            if attr_str.contains("test") {
                self.in_test_module = true;
            }
        }
        visit::visit_attribute(self, node);
    }
}

/// Resolve references to actual symbol IDs in the graph.
fn resolve_references(file: &mut IndexedFile) {
    // Build a map of symbol names to IDs within this file
    let mut name_to_id = HashMap::new();
    for symbol in &file.symbols {
        name_to_id.insert(symbol.name.clone(), symbol.id.clone());
        // Also map qualified names
        if let Some(_parent) = &symbol.id.parent {
            // Skip qualified names for now to avoid compilation issues
        }
    }

    // Resolve references
    for ref_ in &mut file.references {
        if let Some(id) = name_to_id.get(&ref_.symbol_id.name) {
            ref_.symbol_id = id.clone();
        }
    }
}
