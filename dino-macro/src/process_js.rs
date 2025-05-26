use darling::{
    FromDeriveInput, FromField,
    ast::{Data, Style},
};
use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(error_info))]
struct StructData {
    ident: syn::Ident,
    generics: syn::Generics,
    data: Data<(), StructFields>,
}

#[derive(Debug, FromField)]
struct StructFields {
    ident: Option<syn::Ident>,
    ty: syn::Type,
}

pub fn process_from_js(input: syn::DeriveInput) -> TokenStream {
    let (ident, generics, merged, fields) = parse_struct(input);

    let code = fields.iter().map(|field| {
        let name = field.ident.as_ref().expect("Field must have an name");
        let ty = &field.ty;

        let name_str = format!("{}", name);
        quote! {
            let #name: #ty = obj.get(#name_str)?;
        }
    });

    let idents = fields.iter().map(|field| {
        let name = field.ident.as_ref().expect("Field must have an name");
        quote! {
            #name
        }
    });

    quote! {
        impl #merged rquickjs::FromJs<'js> for #ident #generics {
            fn from_js(_ctx: &rquickjs::Ctx<'js>, v: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
                let obj = v.into_object().unwrap();

                #(#code)*

                Ok(#ident {
                    #(#idents),*
                }

                )
            }
        }
    }
}

// impl<'js> IntoJs<'js> for Request {
//     fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
//         let obj = Object::new(ctx.clone())?;
//         obj.set("method", self.method.into_js(ctx)?)?;
//         obj.set("url", self.url.into_js(ctx)?)?;
//         obj.set("headers", self.headers.into_js(ctx)?)?;
//         obj.set("body", self.body.into_js(ctx)?)?;

//         Ok(obj.into())
//     }
// }
pub fn process_into_js(input: syn::DeriveInput) -> TokenStream {
    let (ident, generics, merged, fields) = parse_struct(input);

    let code = fields.iter().map(|field| {
        let name = field.ident.as_ref().expect("Field must have a name");
        let name_str = format!("{}", name);
        quote! {
            obj.set(#name_str, self.#name.into_js(ctx)?)?;
        }
    });

    quote! {
        impl #merged rquickjs::IntoJs<'js> for #ident #generics {
            fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
                let obj = rquickjs::Object::new(ctx.clone())?;
                #(#code)*

                Ok(obj.into())
            }
        }
    }
}

fn parse_struct(
    input: syn::DeriveInput,
) -> (syn::Ident, syn::Generics, syn::Generics, Vec<StructFields>) {
    let StructData {
        ident,
        generics,
        data: Data::Struct(fields),
    } = StructData::from_derive_input(&input).expect("Failed to parse input")
    else {
        panic!("Only structs are supported");
    };

    let fields = match fields.style {
        Style::Struct => fields.fields,
        _ => panic!("Only named fields are supported"),
    };
    let mut merged = generics.clone();

    merged.params.push(syn::parse_quote!('js));

    (ident, generics, merged, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_struct() {
        let input = r#"
          #[derive(FromJs)]
          struct Request {
            method: String,
            url: String,
            headers: HashMap<String, String>,
            body: Option<String>,
          }
        "#;
        let input = syn::parse_str::<syn::DeriveInput>(input).unwrap();

        let (ident, _generics, merged, _fields) = parse_struct(input);

        assert_eq!(ident, "Request");
        // assert_eq!(generics, syn::Generics::default());
        // merged generics is <'js>
        //用<'js>构造一个Generics
        let mut merged_assert = syn::Generics::default();
        merged_assert.params.push(syn::parse_quote!('js));
        assert_eq!(merged, merged_assert);
    }

    #[test]
    fn test_process_from_js_should_work() {
        let input = r#"
            #[derive(FromJs)]
            struct Request {
                method: String,
                url: String,
                headers: HashMap<String, String>,
                body: Option<String>,
            }
        "#;
        let input = syn::parse_str::<syn::DeriveInput>(input).unwrap();

        let tokens = process_from_js(input);

        assert_eq!(tokens.to_string(), quote! {
            impl<'js> rquickjs::FromJs<'js> for Request {
                fn from_js(_ctx: &rquickjs::Ctx<'js>, v: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
                    let obj = v.into_object().unwrap();

                    let method: String = obj.get("method")?;
                    let url: String = obj.get("url")?;
                    let headers: HashMap<String, String> = obj.get("headers")?;
                    let body: Option<String> = obj.get("body")?;

                    Ok(Request { method, url, headers, body })
                }
            }
        }.to_string());
    }

    #[test]
    fn test_process_into_js_should_work() {
        let input = r#"
            #[derive(IntoJs)]
            pub struct Request {
                #[builder(setter(into))]
                pub method: String,
                #[builder(setter(into))]
                pub url: String,
                pub headers: HashMap<String, String>,
                #[builder(default, setter(strip_option))]
                pub body: Option<String>,
            }
        "#;
        let input = syn::parse_str::<syn::DeriveInput>(input).unwrap();
        let tokens = process_into_js(input);
        assert_eq!(tokens.to_string(), quote! {
            impl<'js> rquickjs::IntoJs<'js> for Request {
                fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
                    let obj = rquickjs::Object::new(ctx.clone())?;
                    obj.set("method", self.method.into_js(ctx)?)?;
                    obj.set("url", self.url.into_js(ctx)?)?;
                    obj.set("headers", self.headers.into_js(ctx)?)?;
                    obj.set("body", self.body.into_js(ctx)?)?;

                    Ok(obj.into())
                }
            }
        }.to_string());
    }
}
