extern crate proc_macro2;

use std::fmt::format;
use std::str::from_utf8;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_macro_input, Data::Struct, DataStruct, DeriveInput};
use quote::quote;
use quote::ToTokens;


fn create_macros() -> proc_macro2::TokenStream {
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

        macro_rules! create_builder_each_setter {
            ($fname: ident, $argname: ident, Vec<$ftype: ty>) => {
                pub fn $fname(&mut self, argument: $ftype) -> &mut Self {
                    if self.$argname.is_none() {
                        self.$argname = Some(Vec::new());
                    } 

                    // We assigned this just above
                    self.$argname.as_mut().unwrap().push(argument);
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
            ($fname: ident, Vec<$ftype: ty>) => {
                pub fn $fname(argument: &std::option::Option<Vec<$ftype>>) -> Vec<$ftype> {
                    match argument {
                        Some(arg) => {
                            return arg.clone();
                        }
                        None => {
                            return Vec::new();
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
            ($self: ident, $fname: ident, Vec<$ftype: ty>) => {
            };
            ($self: ident, $fname: ident, $ftype: ty) => {
                // Non optional is set to None... whaaat
                if $self.$fname.is_none() {
                    panic!{"Non-optional param is set to none {}", stringify!{$fname}}
                }
            };
        }


    }
}


fn generate_member_variables_of_builder(my_struct: &DataStruct) -> proc_macro2::TokenStream {
    let fields = my_struct.fields.iter().map(|field| {
        let name = &field.ident; 
        let ty = &field.ty;

        quote! {
            #name: std::option::Option<#ty>
        }
    });

    quote! {
        #(#fields,)*
    }.into()
}


fn generate_setter_functions_of_builder(my_struct: &DataStruct) -> proc_macro2::TokenStream {
    let setters = my_struct.fields.iter().map(|field| {
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

        let ty = &field.ty;

        let each_setter: proc_macro2::TokenStream = match parse_each_attribute(&field.attrs) {
            Ok(optional_str) => {
                match optional_str {
                    Some(string) => {
                        // string looks like : ""each"" - cut off the ""
                        let cut_str: String = match from_utf8(&string.as_bytes()[1..string.len() - 1]) {
                            Ok(s) => {
                                s.to_owned()
                            }
                            Err(e) => {
                                panic!("{}", e);
                            }
                        };
                        
                        let each_name: syn::Ident = syn::Ident::new(&cut_str, Span::call_site());
                        quote! {
                            create_builder_each_setter! (#each_name, #name, #ty);
                        }
                    },
                    None => {
                        quote! {
                            create_builder_setter! (#name, #ty);
                        }
                    }
                }
            },
            Err(error) => {
                panic!("Error parsing attributes: {}", error);
            }
        };

        quote! {
            create_final_builder_unwrap! (#setter_name, #ty);

            #each_setter
        }
    });

    quote! {
        #(#setters)*
    }.into()
}


fn generate_default_setters_for_builder(my_struct: &DataStruct) -> proc_macro2::TokenStream {
    let default_setters = my_struct.fields.iter().map(|field| {
        let name = &field.ident;
        quote! {
            #name: None
        }
    });

    quote! {
        Builder{#(#default_setters,)*}
    }.into()
}


fn generate_setters_for_final_constructor(my_struct: &DataStruct, struct_name: &syn::Ident) -> proc_macro2::TokenStream {
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

    quote! {
        #struct_name{#(#builder,)*}
    }
}

fn generate_validators(my_struct: &DataStruct) -> proc_macro2::TokenStream {
    let validators = my_struct.fields.iter().map(|field| {
        let name = &field.ident;
        let ty = &field.ty;

        quote! {
            validate_field_on_build! (self, #name, #ty);
        }
    });

    quote! {
        #(#validators)*
    }.into()
}


// Looks for each attriute (our attribute)
fn parse_each_attribute(attr_vec: &Vec<syn::Attribute>) -> Result<Option<String>, String> {
    if attr_vec.len() == 0 {
        return Ok(None);
    }

    let attr = &attr_vec[0];

    if attr.path().is_ident("builder") {
        match attr.parse_args() {
            Ok(x) => {
                match x {
                    syn::Meta::NameValue(nv) => {
                        match nv.path.get_ident() {
                            Some(valid_ident) => {
                                if *valid_ident == proc_macro2::Ident::new("each", Span::call_site()) {
                                    return Ok(Some(nv.value.to_token_stream().to_string()));
                                }
                                else {
                                    panic!("expected `builder(each = \"...\")`");
                                }
                            }
                            None => {
                                return Err(format(format_args!("Param ident cannot be None")));
                            }
                        }
                    }
                    _ => {
                        panic!("Unexpected");
                    }
                }
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }

    return Ok(None);
}


#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {

    pub fn gen_builder_str_for_struct(my_struct: DataStruct, struct_name_ident: syn::Ident) -> TokenStream {
        let macros: proc_macro2::TokenStream = create_macros();

        let default_constructor = generate_default_setters_for_builder(&my_struct);
        let fields = generate_member_variables_of_builder(&my_struct);
        let setters = generate_setter_functions_of_builder(&my_struct);
        let validators = generate_validators(&my_struct);
        let final_constructor = generate_setters_for_final_constructor(&my_struct, &struct_name_ident);

        quote! {
            #macros

            impl #struct_name_ident {
                pub fn builder() -> Builder { 
                    let build = #default_constructor; 
                    return build; 
                }
            }

            pub struct Builder {
                #fields
            }

            impl Builder {
                #setters

                pub fn build(&self) -> std::option::Option<#struct_name_ident> {
                    #validators
                    return std::option::Option::Some(#final_constructor);
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

    return description_str;
}
