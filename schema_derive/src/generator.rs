// src/generator.rs

//! 生成结构体内部 JSON Schema 的逻辑

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Data, DataStruct, Fields, Ident, Type};

use crate::attributes::parse_field_attributes;
use crate::type_helpers::{is_option, is_vec, get_option_inner_type, get_vec_inner_type, map_rust_type_to_json};

/// 保存字段信息
pub struct FieldInfo {
    /// 字段标识符
    pub ident: Ident,
    /// 字段类型
    pub ty: Type,
    /// 字段上所有属性
    pub attributes: Vec<syn::Attribute>,
}

/// 从 DeriveInput 中提取具名字段信息
pub fn extract_fields(input: &DeriveInput) -> Vec<FieldInfo> {
    if let Data::Struct(DataStruct { fields: Fields::Named(named_fields), .. }) = &input.data {
        named_fields.named.iter().map(|field| FieldInfo {
            ident: field.ident.clone().expect("字段必须具名"),
            ty: field.ty.clone(),
            attributes: field.attrs.clone(),
        }).collect()
    } else {
        panic!("JsonSchema 只支持具名字段的结构体");
    }
}

/// 根据字段信息生成内部 JSON Schema
pub fn generate_inner_schema(fields: Vec<FieldInfo>) -> TokenStream2 {
    let mut property_entries = quote! {};
    let mut required_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.to_string();
        let field_name_lit = syn::LitStr::new(&field_name, field.ident.span());

        // 解析字段级属性（例如 description、enum、ref、required）
        let field_attrs = parse_field_attributes(&field.attributes);

        // 构造基础 schema：若存在 $ref 优先处理；否则根据 Option/Vec 生成相应结构
        let base_schema = if let Some(ref reference_path) = field_attrs.reference {
            let ref_lit = syn::LitStr::new(reference_path, field.ident.span());
            if is_vec(&field.ty)
                || get_option_inner_type(&field.ty).map_or(false, |ty| is_vec(ty))
            {
                quote! {
                    {
                        let mut field_schema = serde_json::Map::new();
                        field_schema.insert("type".to_string(), serde_json::Value::String("array".to_string()));
                        let mut items = serde_json::Map::new();
                        items.insert("$ref".to_string(), serde_json::Value::String(#ref_lit.to_string()));
                        field_schema.insert("items".to_string(), serde_json::Value::Object(items));
                        field_schema
                    }
                }
            } else {
                quote! {
                    {
                        let mut field_schema = serde_json::Map::new();
                        field_schema.insert("$ref".to_string(), serde_json::Value::String(#ref_lit.to_string()));
                        field_schema
                    }
                }
            }
        } else if is_option(&field.ty) {
            let inner_ty = get_option_inner_type(&field.ty).expect("Option 类型必须有内部类型");
            let (json_type, _json_format) = map_rust_type_to_json(inner_ty);
            let type_lit = syn::LitStr::new(&json_type, field.ident.span());
            quote! {
                {
                    let mut field_schema = serde_json::Map::new();
                    field_schema.insert("type".to_string(), serde_json::Value::Array(vec![
                        serde_json::Value::String(#type_lit.to_string()),
                        serde_json::Value::String("null".to_string())
                    ]));
                    field_schema
                }
            }
        } else if is_vec(&field.ty) {
            let inner_ty = get_vec_inner_type(&field.ty).expect("Vec 类型必须有内部类型");
            let (json_type, json_format) = map_rust_type_to_json(inner_ty);
            let type_lit = syn::LitStr::new(&json_type, field.ident.span());
            let format_lit = syn::LitStr::new(&json_format, field.ident.span());
            quote! {
                {
                    let mut field_schema = serde_json::Map::new();
                    field_schema.insert("type".to_string(), serde_json::Value::String("array".to_string()));
                    let mut items = serde_json::Map::new();
                    items.insert("type".to_string(), serde_json::Value::String(#type_lit.to_string()));
                    if !#format_lit.is_empty() {
                        items.insert("format".to_string(), serde_json::Value::String(#format_lit.to_string()));
                    }
                    field_schema.insert("items".to_string(), serde_json::Value::Object(items));
                    field_schema
                }
            }
        } else {
            let (json_type, json_format) = map_rust_type_to_json(&field.ty);
            let type_lit = syn::LitStr::new(&json_type, field.ident.span());
            let format_lit = syn::LitStr::new(&json_format, field.ident.span());
            quote! {
                {
                    let mut field_schema = serde_json::Map::new();
                    field_schema.insert("type".to_string(), serde_json::Value::String(#type_lit.to_string()));
                    if !#format_lit.is_empty() {
                        field_schema.insert("format".to_string(), serde_json::Value::String(#format_lit.to_string()));
                    }
                    field_schema
                }
            }
        };

        // 根据字段属性扩展 schema，如添加 description 和 enum
        let field_schema = if let Some(ref description) = field_attrs.description {
            let desc_lit = syn::LitStr::new(description, field.ident.span());
            if let Some(enum_values) = field_attrs.enum_values {
                let enum_lits: Vec<syn::LitStr> = enum_values
                    .iter()
                    .map(|val| syn::LitStr::new(val, field.ident.span()))
                    .collect();
                quote! {
                    {
                        let mut field_schema = #base_schema;
                        field_schema.insert("description".to_string(), serde_json::Value::String(#desc_lit.to_string()));
                        let enum_array: Vec<serde_json::Value> = vec![#(#enum_lits),*]
                            .into_iter()
                            .map(|s| serde_json::Value::String(s.to_string()))
                            .collect();
                        field_schema.insert("enum".to_string(), serde_json::Value::Array(enum_array));
                        serde_json::Value::Object(field_schema)
                    }
                }
            } else {
                quote! {
                    {
                        let mut field_schema = #base_schema;
                        field_schema.insert("description".to_string(), serde_json::Value::String(#desc_lit.to_string()));
                        serde_json::Value::Object(field_schema)
                    }
                }
            }
        } else if let Some(enum_values) = field_attrs.enum_values {
            let enum_lits: Vec<syn::LitStr> = enum_values
                .iter()
                .map(|val| syn::LitStr::new(val, field.ident.span()))
                .collect();
            quote! {
                {
                    let mut field_schema = #base_schema;
                    let enum_array: Vec<serde_json::Value> = vec![#(#enum_lits),*]
                        .into_iter()
                        .map(|s| serde_json::Value::String(s.to_string()))
                        .collect();
                    field_schema.insert("enum".to_string(), serde_json::Value::Array(enum_array));
                    serde_json::Value::Object(field_schema)
                }
            }
        } else {
            quote! {
                {
                    let field_schema = #base_schema;
                    serde_json::Value::Object(field_schema)
                }
            }
        };

        property_entries.extend(quote! {
            properties.insert(#field_name_lit.to_string(), #field_schema);
        });

        // 如果字段不是 Option 类型或被强制标记为 required，则加入 required 列表
        if !is_option(&field.ty) || field_attrs.force_required {
            required_fields.push(field_name_lit);
        }
    }

    let required_block = if required_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            schema.insert("required".to_string(), serde_json::Value::Array(
                vec![#(#required_fields),*].into_iter()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect()
            ));
        }
    };

    quote! {
        {
            let mut properties = serde_json::Map::new();
            #property_entries
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            #required_block
            schema.insert("additionalProperties".to_string(), serde_json::Value::Bool(false));
            serde_json::Value::Object(schema)
        }
    }
}

/// 实现 JsonSchema 过程宏的具体逻辑
pub fn json_schema_derive_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use quote::quote;
    use syn::{parse_macro_input, DeriveInput, LitStr};

    let input_ast = parse_macro_input!(input as DeriveInput);
    let struct_attrs = crate::attributes::parse_struct_attributes(&input_ast);
    let fields = extract_fields(&input_ast);
    let inner_schema = generate_inner_schema(fields);

    let schema_tokens = if struct_attrs.inner {
        inner_schema
    } else {
        let name = struct_attrs.name.expect("外层 schema 必须指定 name（例如：#[schema(name = \"xxx\")]）");
        let name_lit = LitStr::new(&name, proc_macro2::Span::call_site());
        let strict = struct_attrs.strict;
        if let Some(desc) = struct_attrs.description {
            let desc_lit = LitStr::new(&desc, proc_macro2::Span::call_site());
            quote! {
                {
                    let mut outer = serde_json::Map::new();
                    outer.insert("type".to_string(), serde_json::Value::String("json_schema".to_string()));
                    let mut inner_obj = serde_json::Map::new();
                    inner_obj.insert("name".to_string(), serde_json::Value::String(#name_lit.to_string()));
                    inner_obj.insert("description".to_string(), serde_json::Value::String(#desc_lit.to_string()));
                    inner_obj.insert("schema".to_string(), #inner_schema);
                    inner_obj.insert("strict".to_string(), serde_json::Value::Bool(#strict));
                    outer.insert("json_schema".to_string(), serde_json::Value::Object(inner_obj));
                    serde_json::Value::Object(outer)
                }
            }
        } else {
            quote! {
                {
                    let mut outer = serde_json::Map::new();
                    outer.insert("type".to_string(), serde_json::Value::String("json_schema".to_string()));
                    let mut inner_obj = serde_json::Map::new();
                    inner_obj.insert("name".to_string(), serde_json::Value::String(#name_lit.to_string()));
                    inner_obj.insert("schema".to_string(), #inner_schema);
                    inner_obj.insert("strict".to_string(), serde_json::Value::Bool(#strict));
                    outer.insert("json_schema".to_string(), serde_json::Value::Object(inner_obj));
                    serde_json::Value::Object(outer)
                }
            }
        }
    };

    let struct_name = &input_ast.ident;
    let expanded = quote! {
        impl JsonSchema for #struct_name {
            fn json_schema() -> serde_json::Value {
                #schema_tokens
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}
