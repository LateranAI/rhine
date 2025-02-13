use crate::path_solver::get_module_path;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, Expr, ExprLit, Ident, ItemFn, Lit, LitStr, Meta,
    Path, Result, Token, Type,
};

#[derive(Default)]
struct ToolAttr {
    name: Option<String>,
    description: Option<String>,
    parameters: Option<String>,
    strict: bool,
    module_path: Option<Path>,
}

fn parse_tool_attributes(args: Punctuated<Meta, Token![,]>) -> Result<ToolAttr> {
    let mut tool_attr = ToolAttr::default();

    for meta in args {
        match meta {
            Meta::NameValue(nv) => {
                let ident = nv
                    .path
                    .get_ident()
                    .ok_or_else(|| syn::Error::new_spanned(&nv, "Expected identifier"))?;

                match ident.to_string().as_str() {
                    "name" | "description" | "parameters" => {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = nv.value
                        {
                            match ident.to_string().as_str() {
                                "name" => tool_attr.name = Some(lit_str.value()),
                                "description" => tool_attr.description = Some(lit_str.value()),
                                "parameters" => tool_attr.parameters = Some(lit_str.value()),
                                _ => unreachable!(),
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                &nv.value,
                                "Expected string literal",
                            ));
                        }
                    }
                    "strict" => {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Bool(lit_bool),
                            ..
                        }) = nv.value
                        {
                            tool_attr.strict = lit_bool.value();
                        } else {
                            return Err(syn::Error::new_spanned(
                                &nv.value,
                                "Expected boolean literal",
                            ));
                        }
                    }
                    "module_path" => {
                        if let Expr::Path(path) = nv.value {
                            tool_attr.module_path = Some(path.path.clone());
                        } else {
                            return Err(syn::Error::new_spanned(&nv.value, "Expected path"));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            ident,
                            "Unknown attribute parameter",
                        ))
                    }
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    quote::quote! {}, // This produces a valid token stream
                    "Unsupported attribute format",
                ))
            }
        }
    }

    if tool_attr.parameters.is_none() {
        return Err(syn::Error::new_spanned(
            quote::quote! {}, // This produces a valid token stream
            "Missing required 'parameters' attribute",
        ));
    }

    Ok(tool_attr)
}

fn camel_to_snake(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.extend(ch.to_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

pub fn function_tool_attr_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);

    let tool_attr = match parse_tool_attributes(args) {
        Ok(attr) => attr,
        Err(e) => return e.to_compile_error().into(),
    };

    let module_path = tool_attr
        .module_path
        .unwrap_or_else(|| syn::parse_str(&get_module_path(&input_fn).unwrap()).unwrap());

    let fn_name = input_fn.sig.ident.clone();
    let fn_name_str = fn_name.to_string();
    let tool_name = tool_attr
        .name
        .unwrap_or_else(|| camel_to_snake(&fn_name_str));
    let tool_name_lit = LitStr::new(&tool_name.clone(), fn_name.span());
    let description = tool_attr.description.unwrap_or_default();
    let description_lit = LitStr::new(&description, fn_name.span());
    let parameters_type_str = tool_attr.parameters.unwrap();
    let parameters_type: Type = syn::parse_str(&parameters_type_str).unwrap();
    let strict = tool_attr.strict;

    let tool_schema_fn_name = Ident::new(&format!("{}_tool_schema", fn_name_str), fn_name.span());
    let init_module_name = format_ident!("__init_{}", tool_name);

    let expanded = quote! {
        #input_fn

        #[allow(non_snake_case)]
        pub fn #tool_schema_fn_name() -> serde_json::Value {
            let mut params_schema = <#module_path::#parameters_type as JsonSchema>::json_schema();

            let mut tool_obj = serde_json::Map::new();
            tool_obj.insert("name".to_string(), serde_json::Value::String(#tool_name_lit.to_string()));
            tool_obj.insert("description".to_string(), serde_json::Value::String(#description_lit.to_string()));
            tool_obj.insert("parameters".to_string(), params_schema);
            tool_obj.insert("strict".to_string(), serde_json::Value::Bool(#strict));

            let mut outer = serde_json::Map::new();
            outer.insert("type".to_string(), serde_json::Value::String("function".to_string()));
            outer.insert("function".to_string(), serde_json::Value::Object(tool_obj));
            serde_json::Value::Object(outer)
        }

        mod #init_module_name {
            #[used]
            #[link_section = ".CRT$XCU"]
            static INIT: extern "C" fn() = {
                extern "C" fn initialize() {
                    use std::sync::Arc;
                    use error_stack::{Result, ResultExt, Report};
                    use crate::utils::chat::function_calling::get_tool_registry;
                    use crate::utils::chat::function_calling::FunctionCallingError;

                    let tool_name = #tool_name_lit.to_string();
                    let tool_name_clone = tool_name.clone();
                    let wrapper = move |params: serde_json::Value| -> _ {
                        let parsed_params: #module_path::#parameters_type = serde_json::from_value(
                            params.clone()
                        ).map_err(|e| {
                            Report::new(
                                FunctionCallingError::ParamsParseError(
                                    tool_name.clone(),
                                    params.to_string()
                                )
                            )
                        })?;
                        let result = #module_path::#fn_name(parsed_params);
                        serde_json::to_value(result).map_err(|e| {
                            Report::new(
                                FunctionCallingError::ResultParseError(
                                    tool_name.clone(),
                                )
                            )
                        })
                    };

                    get_tool_registry().insert(tool_name_clone, Arc::new(wrapper));
                }
                initialize
            };
        }
    };

    TokenStream::from(expanded)
}
