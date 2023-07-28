use proc_macro::{TokenStream};
use std::fmt::{Debug, Formatter};
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, ToTokens};
use syn::{FnArg, ImplItem, ItemImpl, parse_macro_input, Pat, ReturnType, Type};

struct Item {
    name: String,
    enum_name: String,
    inputs: Vec<Box<Type>>,
    safe_input_names: Vec<Ident>,
    input_names: Vec<Ident>,
    is_async: bool,
    output: proc_macro2::TokenStream,
}

impl Debug for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Item")
            .field("name", &self.name)
            .field("enum_name", &self.enum_name)
            .finish_non_exhaustive()
    }
}

/// attribute to create a command enum for an impl block, currently limited to structs without any generic parameters
///
/// Functions can't have generic parameters either, and all parameters should be [`Send`](core::marker::Send)
#[proc_macro_attribute]
pub fn performer(_tag: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemImpl);


    let (receiver_name_str, receiver_name, command_name, facade_name, facade_mut_name) = match &*input.self_ty {
        Type::Path(p) => {
            let receiver_name = p.path.to_token_stream().to_string();

            let mut command_path = p.path.clone();
            if let Some(v) = command_path.segments.last_mut() {
                v.ident = Ident::new(&format!("{}Command", v.ident.to_string()), Span::call_site());

                if !v.arguments.is_none() {
                    panic!("{receiver_name} has generic parameters which brug doesnt support yet");
                }
            }

            let mut facade_path = p.path.clone();
            if let Some(v) = facade_path.segments.last_mut() {
                v.ident = Ident::new(&format!("{}Facade", v.ident.to_string()), Span::call_site());
            }

            let mut facade_mut_path = p.path.clone();
            if let Some(v) = facade_mut_path.segments.last_mut() {
                v.ident = Ident::new(&format!("{}FacadeMut", v.ident.to_string()), Span::call_site());
            }


            (receiver_name, p.path.clone(), command_path, facade_path, facade_mut_path)
        }
        _ => {
            panic!("don't understand impl block")
        }
    };

    let mut items = vec![];
    for item in input.items.clone() {
        match item {
            ImplItem::Fn(func) => {
                let is_async = func.sig.asyncness.is_some();
                let name = func.sig.ident.to_string();
                let enum_name = change_casing(&name);
                let mut inputs = vec![];
                let mut safe_input_names = vec![];
                let mut input_names = vec![];

                if !func.sig.generics.params.is_empty() {
                    panic!("{}::{}() has generic parameters, which can't be supported", receiver_name_str, name);
                }

                if !func.sig.variadic.is_none() {
                    panic!("{}::{}() is variadic parameters, which can't be supported", receiver_name_str, name);
                }


                let mut i = 0;
                for input in func.sig.inputs {
                    match input {
                        FnArg::Receiver(_) => {}
                        FnArg::Typed(t) => {
                            inputs.push(t.ty);
                            safe_input_names.push(format_ident!("arg_{i}"));
                            input_names.push(match &*t.pat {
                                Pat::Ident(id) => id.ident.clone(),
                                _ => format_ident!("arg_{i}"),
                            });
                            i += 1;
                        }
                    }
                }

                let output = match func.sig.output {
                    ReturnType::Default => quote! { () },
                    ReturnType::Type(_, t) => t.to_token_stream(),
                };

                items.push(Item {
                    name,
                    enum_name,
                    inputs,
                    safe_input_names,
                    input_names,
                    is_async,
                    output,
                })
            }
            _ => {}
        }
    }

    let mut facade_errors = vec![];
    let mut names = vec![];
    let mut enum_names = vec![];
    let mut inputs = vec![];
    let mut input_names = vec![];
    let mut safe_input_names = vec![];
    let mut outputs = vec![];
    let mut awaits = vec![];

    for item in items {
        facade_errors.push(format!("performer for {} didn't return a value", item.name));
        names.push(Ident::new(&item.name, Span::call_site()));
        enum_names.push(Ident::new(&item.enum_name, Span::call_site()));
        inputs.push(item.inputs);
        input_names.push(item.input_names);
        safe_input_names.push(item.safe_input_names);
        outputs.push(item.output);

        if item.is_async {
            awaits.push(quote! { .await });
        } else {
            awaits.push(quote! {});
        }
    }

    let d = quote! {
        #input

        pub enum #command_name<T: ::brug::Transport> {
            #(#enum_names(#(#inputs,)* T::Sender<#outputs>)),*
        }

        #[::brug::async_trait]
        impl<T: ::brug::Transport> ::brug::Performer<#command_name<T>> for #receiver_name {
            async fn perform(&mut self, command: #command_name<T>) {
                match command {
                    #(
                        #command_name::#enum_names(#(#safe_input_names,)* resp) => {
                            ::brug::Sender::send(resp, self.#names(#(#safe_input_names),*)#awaits).await;
                        }
                    )*
                }
            }
        }

        #[::brug::async_trait]
        pub trait #facade_name<T: ::brug::Transport> {
            #(
                async fn #names(&self, #(#input_names: #inputs),*) -> #outputs {
                    let (__brug_s, __brug_r) = T::pair();
                    self.handle(#command_name::#enum_names(#(#input_names,)* __brug_s)).await;
                    ::brug::Receiver::receive(__brug_r).await.expect(#facade_errors)
                }
            )*

            async fn handle(&self, command: #command_name<T>);
        }

        #[::brug::async_trait]
        pub trait #facade_mut_name<T: ::brug::Transport> {
            #(
                async fn #names(&mut self, #(#input_names: #inputs),*) -> #outputs {
                    let (__brug_s, __brug_r) = T::pair();
                    self.handle(#command_name::#enum_names(#(#input_names,)* __brug_s)).await;
                    ::brug::Receiver::receive(__brug_r).await.expect(#facade_errors)
                }
            )*

            async fn handle(&mut self, command: #command_name<T>);
        }

        #[::brug::async_trait]
        impl<T: ::brug::Transport, F: #facade_name<T> + Send + Sync> #facade_mut_name<T> for F {
            async fn handle(&mut self, command: #command_name<T>) {
                #facade_name::handle(self, command).await;
            }
        }
    };

    d.into()
}


fn change_casing(input: &str) -> String {
    let mut x = String::with_capacity(input.len());
    for item in input.split("_") {
        let mut c = item.chars();
        if let Some(first) = c.next() {
            x.push(first.to_ascii_uppercase());
            x.extend(c);
        } else {
            x.push('_');
        }
    }

    x
}