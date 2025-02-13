// src/type_helpers.rs

//! 类型辅助工具：判断是否为 Option/Vec 以及将 Rust 类型映射为 JSON Schema 的 type 和 format

use syn::{Type, TypePath, PathArguments, GenericArgument};

/// 判断给定类型是否为 Option<T>
pub fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.segments.iter().any(|seg| seg.ident == "Option")
    } else {
        false
    }
}

/// 判断给定类型是否为 Vec<T>
pub fn is_vec(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.segments.iter().any(|seg| seg.ident == "Vec")
    } else {
        false
    }
}

/// 如果类型为 Option<T>，则返回内部 T 类型；否则返回 None
pub fn get_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let seg = path.segments.last()?;
        if seg.ident != "Option" {
            return None;
        }
        if let PathArguments::AngleBracketed(ref args) = seg.arguments {
            if let Some(GenericArgument::Type(inner)) = args.args.first() {
                return Some(inner);
            }
        }
    }
    None
}

/// 如果类型为 Vec<T>，则返回内部 T 类型；否则返回 None
pub fn get_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let seg = path.segments.last()?;
        if seg.ident != "Vec" {
            return None;
        }
        if let PathArguments::AngleBracketed(ref args) = seg.arguments {
            if let Some(GenericArgument::Type(inner)) = args.args.first() {
                return Some(inner);
            }
        }
    }
    None
}

/// 将 Rust 类型映射为 JSON Schema 的 type 与可能的 format
/// 例如，String -> "string"，i32 -> "integer"，f64 -> "number"，bool -> "boolean"
pub fn map_rust_type_to_json(ty: &Type) -> (String, String) {
    let type_str = match ty {
        Type::Path(type_path) => {
            let seg = type_path.path.segments.last().unwrap();
            match seg.ident.to_string().as_str() {
                "String" => "string",
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => "integer",
                "f32" | "f64" => "number",
                "bool" => "boolean",
                _ => "object",
            }
        }
        _ => "object",
    };
    (type_str.to_string(), "".to_string())
}
