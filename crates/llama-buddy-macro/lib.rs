use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(IndexByField)]
pub fn derive_index_by_field(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // 获取结构体名称
    let type_name = &input.ident;
    // 获取字段
    let match_arms: Vec<_> = match input.data {
        Data::Struct(data_struct) => {
            if let Fields::Named(fields) = &data_struct.fields {
                fields
                    .named
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        quote! {
                            #field_name => #index,
                        }
                    })
                    .collect()
            } else {
                panic!("IndexByField can only be derived for structs with named fields");
            }
        }
        Data::Enum(data_enum) => data_enum
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                let variant_name = variant.ident.to_string();
                quote! {
                    #variant_name => #index,
                }
            })
            .collect(),
        Data::Union(data_union) => data_union
            .fields
            .named
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                quote! {
                    #field_name => #index,
                }
            })
            .collect(),
    };

    let expanded = quote! {
        impl #type_name {
            pub fn index_by_field(name: impl AsRef<str>) -> usize {
                let name = name.as_ref();
                match name {
                    #(#match_arms)*
                    _ => panic!("Field not found"),
                }
            }
        }
    };

    TokenStream::from(expanded)
}
