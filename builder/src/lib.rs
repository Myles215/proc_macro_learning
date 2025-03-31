extern crate proc_macro2;

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data::Struct, DataStruct, DeriveInput};
use quote::quote;



#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {

    pub fn create_macros() -> proc_macro2::TokenStream {
        quote!{
            // This macro creates a setter for our builder.
            // e.g. if our builder wants a type of Option
            // we use this macro to accept the T from Option<T>
            macro_rules! create_builder_setter {
                ($fname: ident, Option<$ftype: ty>) => {
                    pub fn $fname(&mut self, argument: $ftype) -> &mut Self {
                        self.$fname = Some(Some(argument));
                        self
                    }
                };
                ($fname: ident, $ftype: ty) => {
                    pub fn $fname(&mut self, argument: $ftype) -> &mut Self {
                        self.$fname = Some(argument);
                        self
                    }
                };
            }

            macro_rules! create_final_builder_unwrap {
                ($fname: ident, Option<$ftype: ty>) => {
                    pub fn $fname(argument: &std::option::Option<std::option::Option<$ftype>>) -> std::option::Option<$ftype> {
                        match argument {
                            Some(arg) => {
                                return arg.clone();
                            }
                            None => {
                                return None;
                            }
                        }
                    }
                };
                ($fname: ident, $ftype: ty) => {
                    pub fn $fname(argument: &std::option::Option<$ftype>) -> $ftype {
                        argument.as_ref().unwrap().clone()
                    }
                };
            }

            // We use this macro to check if a non-optional
            // value is set and just return None otherwise
            macro_rules! validate_field_on_build {
                ($self: ident, $fname: ident, Option<$ftype: ty>) => {
                };
                ($self: ident, $fname: ident, $ftype: ty) => {
                    // Non optional is set to None... whaaat
                    if $self.$fname.is_none() {
                        return None
                    }
                };
            }


        }
    }

    pub fn gen_builder_str_for_struct(my_struct: DataStruct, struct_name_ident: syn::Ident) -> TokenStream {
        let fields = my_struct.fields.iter().map(|field| {
            let name = &field.ident; 
            let ty = &field.ty;

            quote! {
                #name: std::option::Option<#ty>
            }
        });

        let setters = my_struct.fields.iter().map(|field| {
            let name = &field.ident;
            
            if field.attrs.len() > 0 {
                panic!("{:?}", field.attrs);
            }

            let name_as_str = match name {
                Some(thang) => {
                    thang.to_string()
                },  
                None => {
                    "".to_owned()
                }
            };
            let setter_name: proc_macro2::TokenStream = format!("{}_setter", name_as_str).parse().unwrap();

            let ty = &field.ty;

            quote! {
                create_final_builder_unwrap! (#setter_name, #ty);

                create_builder_setter! (#name, #ty);
            }
        });

        let default_setters = my_struct.fields.iter().map(|field| {
            let name = &field.ident;
            quote! {
                #name: None
            }
        });

        let builder = my_struct.fields.iter().map(|field| {
            let name = &field.ident;
            let name_as_str = match name {
                Some(thang) => {
                    thang.to_string()
                },  
                None => {
                    "".to_owned()
                }
            };
            let setter_name: proc_macro2::TokenStream = format!("{}_setter", name_as_str).parse().unwrap();

            quote! {
                #name: Self::#setter_name(&self.#name)
            }
        });

        let validators = my_struct.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote! {
                validate_field_on_build! (self, #name, #ty);
            }
        });

        let macros: proc_macro2::TokenStream = create_macros();

        quote! {
            #macros

            impl #struct_name_ident {
                pub fn builder() -> Builder { 
                    let build = Builder{#(#default_setters,)*}; 
                    return build; 
                }
            }

            pub struct Builder {
                #(#fields,)*
            }

            impl Builder {
                #(#setters)*

                pub fn build(&self) -> std::option::Option<#struct_name_ident> {
                    #(#validators)*
                    return std::option::Option::Some(#struct_name_ident{#(#builder,)*});
                }
            }
        }.into()
    }

    let DeriveInput {
        ident: struct_name_ident,
        data,
        ..
    } = parse_macro_input!(input as DeriveInput);

    let description_str = match data {
        Struct(my_struct) => gen_builder_str_for_struct(my_struct, struct_name_ident),
        _ => TokenStream::new(),
    };

   //panic!("{}", description_str);

    return description_str;
}
