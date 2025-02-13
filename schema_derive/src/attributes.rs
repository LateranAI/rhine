// src/attributes.rs

//! 解析 #[schema(...)] 属性。
//!
//! 分为两部分：
//! - 结构体级属性（例如 name、description、strict、inner）
//! - 字段级属性（例如 desc、enum、ref、required）

use syn::{DeriveInput, Attribute, LitBool, LitStr};

/// 结构体级 schema 属性配置
pub struct StructSchemaAttributes {
    /// 外层 schema 的名称（例如用于工具注册时使用）
    pub name: Option<String>,
    /// 结构体的描述
    pub description: Option<String>,
    /// 是否开启严格模式
    pub strict: bool,
    /// 是否仅生成内部 schema（不包装外层 json_schema 对象）
    pub inner: bool,
}

/// 解析结构体上的 schema 属性
pub fn parse_struct_attributes(input: &DeriveInput) -> StructSchemaAttributes {
    let mut attrs = StructSchemaAttributes {
        name: None,
        description: None,
        strict: false,
        inner: false,
    };

    for attr in &input.attrs {
        if !attr.path().is_ident("schema") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                attrs.name = Some(lit.value());
            } else if meta.path.is_ident("description") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                attrs.description = Some(lit.value());
            } else if meta.path.is_ident("strict") {
                let value = meta.value()?;
                let lit: LitBool = value.parse()?;
                attrs.strict = lit.value();
            } else if meta.path.is_ident("inner") {
                if let Ok(lit) = meta.value()?.parse::<LitBool>() {
                    attrs.inner = lit.value();
                } else {
                    attrs.inner = true;
                }
            }
            Ok(())
        });
    }

    attrs
}

/// 字段级 schema 属性配置
#[derive(Default)]
pub struct FieldAttributes {
    /// 字段描述
    pub description: Option<String>,
    /// 枚举值列表（多个值用逗号分隔）
    pub enum_values: Option<Vec<String>>,
    /// 生成 $ref 时指定的引用路径
    pub reference: Option<String>,
    /// 强制标记字段为 required
    pub force_required: bool,
}

/// 解析字段上的 schema 属性
pub fn parse_field_attributes(attrs: &[Attribute]) -> FieldAttributes {
    let mut field_attrs = FieldAttributes::default();

    for attr in attrs {
        if !attr.path().is_ident("schema") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("desc") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                field_attrs.description = Some(lit.value());
            } else if meta.path.is_ident("enum") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                let parts = lit.value()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                field_attrs.enum_values = Some(parts);
            } else if meta.path.is_ident("ref") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                field_attrs.reference = Some(lit.value());
            } else if meta.path.is_ident("required") {
                let value = meta.value()?;
                let lit: LitBool = value.parse()?;
                field_attrs.force_required = lit.value();
            }
            Ok(())
        });
    }

    field_attrs
}
