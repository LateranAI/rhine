// schema_derive/src/path_resolver.rs
use syn::{Attribute, ItemFn};

pub fn get_module_path(item: &ItemFn) -> syn::Result<String> {
    // 尝试从自定义属性中获取路径
    let path = find_module_path_attr(&item.attrs).unwrap();
    Ok(path)

    // // 从环境变量中获取 crate 路径并推断模块路径
    // let crate_root = std::env::var("CARGO_MANIFEST_DIR")
    //     .map_err(|_| syn::Error::new(Span::call_site(), "无法获取CARGO_MANIFEST_DIR"))?;
    //
    // // 假设你传入了一个文件路径，这里直接传递
    // let file_path = std::path::Path::new(&crate_root).join("src").join("your_module.rs");
    //
    // infer_module_path(&file_path)
}

fn find_module_path_attr(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if attr.path().is_ident("module_path") {
            attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
        } else {
            None
        }
    })
}

// fn infer_module_path(file_path: &std::path::Path) -> syn::Result<String> {
//     let crate_root = std::env::var("CARGO_MANIFEST_DIR")
//         .map_err(|_| syn::Error::new(Span::call_site(), "无法获取CARGO_MANIFEST_DIR"))?;
//
//     let relative_path = file_path.strip_prefix(crate_root)
//         .map_err(|_| syn::Error::new(Span::call_site(), "路径推断失败"))?;
//
//     let module_path = relative_path.with_extension("")
//         .to_string_lossy()
//         .replace(std::path::MAIN_SEPARATOR, "::");
//
//     Ok(format!("crate::{}", module_path))
// }