use proc_macro2::{
    Ident,
    TokenStream,
    TokenTree,
    Delimiter,
    Span,
};
use quote::{
    TokenStreamExt,
    quote
};

use indexmap::IndexMap;
use std::collections::{
    BTreeMap,
    BTreeSet,
};

#[proc_macro]
pub fn create_constrainer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    let mut trees = input.into_iter();
    let name = if let TokenTree::Ident(name) = trees.next().unwrap() {
        name
    } else {
        panic!("Environment needs a name!");
    };

    let data = if let TokenTree::Group(data) = trees.next().unwrap() {
        data
    } else {
        panic!("Need data");
    };
    // println!("{:#?}", data);

    let mut identifiers: IndexMap<Ident, Identifier> = IndexMap::new();

    let mut dynamic_fields = TokenStream::new();
    let mut constrained_fields = TokenStream::new();
    let mut external_fields = TokenStream::new();
    let mut deliminated_dynamics = TokenStream::new();
    let mut deliminated_constraineds = TokenStream::new();
    let mut init_constraineds = TokenStream::new();
    let mut ops: TokenStream = TokenStream::new();

    let mut parse_state = ParseState::Key;
    for token in data.stream() {
        match parse_state {
            ParseState::Key => match token {
                TokenTree::Ident(ident) if ident.to_string().as_str() == "dynamic" => parse_state = ParseState::DynamicName,
                TokenTree::Ident(ident) if ident.to_string().as_str() == "constrained" => parse_state = ParseState::ConstrainedName,
                TokenTree::Ident(ident) if ident.to_string().as_str() == "external" => parse_state = ParseState::ExternalName,
                TokenTree::Ident(ident) if ident.to_string().as_str() == "listener" => parse_state = ParseState::ListenerName,
                TokenTree::Ident(ident) if ident.to_string().as_str() == "opgenset" => parse_state = ParseState::OpGenSet,
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::DynamicName => match token {
                TokenTree::Ident(name) => parse_state = ParseState::DynamicType(name),
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::DynamicType(name) => match token {
                TokenTree::Ident(ty) => {
                    let get_fn_name = Ident::new(&format!("get_{}", name), Span::call_site());
                    ops.append_all(quote! {
                        pub fn #get_fn_name(&self) -> &#ty {
                            &self.#name
                        }
                    });
                    dynamic_fields.append_all(quote! {
                        #name: #ty,
                    });
                    deliminated_dynamics.append_all(quote! {
                        #name,
                    });
                    identifiers.insert(name, Identifier::Dynamic(Dynamic {
                        ty,
                        dependents: BTreeSet::new(),
                    }));
                    parse_state = ParseState::Key;
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ConstrainedName => match token {
                TokenTree::Ident(name) => parse_state = ParseState::ConstrainedType(name),
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ConstrainedType(name) => match token {
                TokenTree::Ident(ty) => {
                    parse_state = ParseState::ConstrainedParams(name, ty);
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ConstrainedParams(name, ty) => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                    let mut params = Vec::new();
                    for token in group.stream() {
                        match token {
                            TokenTree::Punct(punct) if punct.as_char() == ',' => {}, // TODO: Remove >1 comma, no comma, and leading comma
                            TokenTree::Ident(param) => params.push(param),
                            _ => panic!("Unexpected token: {}", token)
                        }
                    }
                    parse_state = ParseState::ConstrainedBlock(name, ty, params);
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ConstrainedBlock(name, ty, params) => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    let index = identifiers.len();
                    let mut compute_args = TokenStream::new();
                    let mut init_args = TokenStream::new();
                    for param in &params {
                        let identifier = identifiers.get_mut(param).unwrap();
                        let param_ty = match identifier {
                            Identifier::Dynamic(dynamic) => {
                                dynamic.dependents.insert(index);
                                &dynamic.ty
                            },
                            Identifier::Constrained(constrained) => {
                                constrained.dependents.insert(index);
                                &constrained.ty
                            },
                            Identifier::External(external) => {
                                external.dependents.insert(index);
                                &external.ty
                            },
                            Identifier::Listener(_) => {
                                panic!("A constrained cannot depend on a listener.");
                            },
                        };
                        compute_args.append_all(quote! {
                            #param: #param_ty,
                        });
                        init_args.append_all(quote! {
                            #param,
                        });
                    }

                    let get_fn_name = Ident::new(&format!("get_{}", name), Span::call_site());
                    ops.append_all(quote! {
                        pub fn #get_fn_name(&self) -> &#ty {
                            &self.#name
                        }
                    });
                    constrained_fields.append_all(quote! {
                        #name: #ty,
                    });
                    deliminated_constraineds.append_all(quote! {
                        #name,
                    });
                    let compute_fn_name = Ident::new(&format!("compute_{}", name), Span::call_site());
                    init_constraineds.append_all(quote! {
                        let #name = Self::#compute_fn_name(#init_args);
                    });
                    let block = group.stream();
                    ops.append_all(quote! { fn #compute_fn_name (#compute_args) -> #ty { #block }});
                    identifiers.insert(name, Identifier::Constrained(Constrained {
                        ty,
                        params,
                        block,
                        compute_fn_name,
                        dependents: BTreeSet::new(),
                    }));
                    parse_state = ParseState::Key;
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ExternalName => match token {
                TokenTree::Ident(name) => parse_state = ParseState::ExternalType(name),
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ExternalType(name) => match token {
                TokenTree::Ident(ty) => {
                    external_fields.append_all(quote! {
                        #name: #ty,
                    });
                    identifiers.insert(name, Identifier::External(External {
                        ty,
                        dependents: BTreeSet::new(),
                    }));
                    parse_state = ParseState::Key;
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ListenerName => match token {
                TokenTree::Ident(name) => parse_state = ParseState::ListenerParams(name),
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ListenerParams(name) => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                    let params = Param::parse_params(group.stream());
                    parse_state = ParseState::ListenerBlock(name, params);
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ListenerBlock(listener_fn_name, params) => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    let index = identifiers.len();
                    let mut listener_args = TokenStream::new();
                    let mut init_args = TokenStream::new();
                    for param in &params {
                        let param_name = &param.name;
                        let identifier = identifiers.get_mut(param_name).unwrap();
                        let param_ty = match identifier {
                            Identifier::Dynamic(dynamic) => {
                                dynamic.dependents.insert(index);
                                &dynamic.ty
                            },
                            Identifier::Constrained(constrained) => {
                                constrained.dependents.insert(index);
                                &constrained.ty
                            },
                            Identifier::External(external) => {
                                external.dependents.insert(index);
                                &external.ty
                            },
                            Identifier::Listener(_) => {
                                panic!("A listener cannot depend on a listener.");
                            },
                        };
                        if param.is_ref {
                            listener_args.append_all(quote! {
                                #param_name: &#param_ty,
                            });
                            init_args.append_all(quote! {
                                &#param_name,
                            });
                        } else {
                            listener_args.append_all(quote! {
                                #param_name: #param_ty,
                            });
                            init_args.append_all(quote! {
                                #param_name,
                            });
                        }
                    }

                    init_constraineds.append_all(quote! {
                        Self::#listener_fn_name(#init_args);
                    });
                    let block = group.stream();
                    ops.append_all(quote! { fn #listener_fn_name (#listener_args) { #block }});
                    identifiers.insert(listener_fn_name, Identifier::Listener(Listener {
                        params,
                        block,
                    }));
                    parse_state = ParseState::Key;
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::OpGenSet => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                    let mut set_dynamics = BTreeMap::new();
                    for token in group.stream() {
                        match token {
                            TokenTree::Punct(punct) if punct.as_char() == ',' => {}, // TODO: Remove >1 comma, no comma, and leading comma
                            TokenTree::Ident(ident) => {
                                let index = identifiers.get_index_of(&ident).unwrap();
                                let key_val = identifiers.get_index(index).unwrap();
                                let name = key_val.0;
                                let dynamic = if let Identifier::Dynamic(dynamic) = key_val.1 {
                                    dynamic
                                } else {
                                    panic!("OpGenSet can only take dynamics");
                                };
                                set_dynamics.insert(index, (name, dynamic));
                            },
                            _ => panic!("Unexpected token: {}", token)
                        }
                    }
                    if set_dynamics.len() == 0 {
                        todo!("opgenset needs at least one dynamic");
                    }

                    let mut set_dynamics_iter = set_dynamics.iter();
                    let mut fn_name = format!("set_{}", set_dynamics_iter.next().unwrap().1.0);
                    for (_, (name, _)) in set_dynamics_iter{
                        fn_name.push_str(&format!("_{}", name));
                    }
                    let fn_name = Ident::new(&fn_name, Span::call_site());

                    let mut set_fn_args = TokenStream::new();
                    let mut fn_block = TokenStream::new();
                    set_fn_args.append_all(quote! { &mut self, });
                    for (_, (name, dynamic)) in &set_dynamics {
                        let ty = &dynamic.ty;
                        set_fn_args.append_all(quote! {
                            #name: #ty,
                        });
                        fn_block.append_all(quote! {
                            self.#name = #name;
                        });
                    }
                    
                    let mut to_update = BTreeMap::new();
                    let mut to_call = BTreeMap::new();
                    let mut found_new_matches = false;
                    for (_, (_, dynamic)) in &set_dynamics {
                        for dependent in &dynamic.dependents {
                            match identifiers.get_index(*dependent).unwrap() {
                                (name, Identifier::Constrained(constrained)) => {
                                    to_update.insert(*dependent, (name, constrained));
                                    
                                },
                                (name, Identifier::Listener(listener)) => {
                                    to_call.insert(*dependent, (name, listener));
                                },
                                _ => unreachable!()
                            }
                            found_new_matches = true;
                        }
                    }

                    if found_new_matches {
                        let mut last_updates_and_call_size = to_update.len()+to_call.len();
                        let mut new_updates = BTreeMap::new();
                        let mut new_calls = BTreeMap::new();
                        for (_, (_, queued)) in &to_update {
                            for dependent in &queued.dependents {
                                match identifiers.get_index(*dependent).unwrap() {
                                    (name, Identifier::Constrained(constrained)) => {
                                        new_updates.insert(*dependent, (name, constrained));
                                    },
                                    (name, Identifier::Listener(listener)) => {
                                        new_calls.insert(*dependent, (name, listener));
                                    },
                                    _ => unreachable!()
                                }
                            }
                        }
                        to_update.append(&mut new_updates);
                        to_call.append(&mut new_calls);
                        
                        while last_updates_and_call_size != to_update.len()+to_call.len() {
                            let mut new_updates = BTreeMap::new();
                            for (_, (_, queued)) in &to_update {
                                for dependent in &queued.dependents {
                                    match identifiers.get_index(*dependent).unwrap() {
                                        (name, Identifier::Constrained(constrained)) => {
                                            new_updates.insert(*dependent, (name, constrained));
                                        },
                                        (name, Identifier::Listener(listener)) => {
                                            new_calls.insert(*dependent, (name, listener));
                                        },
                                        _ => unreachable!()
                                    }
                                }
                            }
                            last_updates_and_call_size = to_update.len()+to_call.len();
                            to_update.append(&mut new_updates);
                            to_call.append(&mut new_calls);
                        }
                    }

                    let mut set_fn_external_args = BTreeMap::new();

                    for (_, (name, constrained)) in to_update {
                        let compute_fn_name = &constrained.compute_fn_name;
                        let mut compute_fn_args = TokenStream::new();
                        for param in &constrained.params {
                            let param_index = identifiers.get_index_of(param).unwrap();
                            match identifiers.get(param).unwrap() {
                                Identifier::Dynamic(_) | Identifier::Constrained(_) => {
                                    compute_fn_args.append_all(quote! {
                                        self.#param,
                                    });
                                },
                                Identifier::External(External {
                                    ty,
                                    ..
                                }) => {
                                    set_fn_external_args.insert(param_index, quote! {
                                        #param: #ty,
                                    });
                                    compute_fn_args.append_all(quote! {
                                        #param,
                                    });
                                },
                                Identifier::Listener(_) => unreachable!()
                            }
                        }
                        fn_block.append_all(quote! {
                            self.#name = Self::#compute_fn_name(#compute_fn_args);
                        });
                    }
                    for (_, (listener_fn_name, listener)) in to_call {
                        let mut listener_fn_args = TokenStream::new();
                        for param in &listener.params {
                            let param_name = &param.name;
                            let param_index = identifiers.get_index_of(param_name).unwrap();
                            match identifiers.get(param_name).unwrap() {
                                Identifier::Dynamic(_) | Identifier::Constrained(_) => {
                                    listener_fn_args.append_all(
                                        if param.is_ref {
                                            quote! {
                                                &self.#param_name,
                                            }
                                        } else {
                                            quote! {
                                                self.#param_name,
                                            }
                                        }
                                    );
                                },
                                Identifier::External(External {
                                    ty,
                                    ..
                                }) => {
                                    set_fn_external_args.insert(param_index, quote! {
                                        #param_name: #ty,
                                    });
                                    listener_fn_args.append_all(
                                        if param.is_ref {
                                            quote! {
                                                &#param_name,
                                            }
                                        } else {
                                            quote! {
                                                #param_name,
                                            }
                                        }
                                    );
                                },
                                Identifier::Listener(_) => unreachable!()
                            }
                        }
                        fn_block.append_all(quote! {
                            Self::#listener_fn_name(#listener_fn_args);
                        });
                    }

                    for (_, set_fn_external_arg) in set_fn_external_args {
                        set_fn_args.append_all(set_fn_external_arg);
                    }

                    ops.append_all(quote! {
                        pub fn #fn_name(#set_fn_args) {
                            #fn_block
                        }
                    });

                    parse_state = ParseState::Key;
                },
                _ => panic!("Unexpected token: {}", token)
            },
        }
    }

    let mut out = TokenStream::new();

    out.append_all(quote! {
        #[derive(Debug)]
        struct #name {
            #dynamic_fields
            #constrained_fields
        }

        impl #name {
            pub fn new ( #dynamic_fields #external_fields ) -> Self {
                #init_constraineds

                Self {
                    #deliminated_dynamics
                    #deliminated_constraineds
                }
            }

            #ops
        }
    });

    // println!("{:#}", out);

    out.into()
}

#[derive(Debug)]
enum ParseState {
    Key,
    DynamicName,
    DynamicType(Ident),
    ConstrainedName,
    ConstrainedType(Ident),
    ConstrainedParams(Ident, Ident),
    ConstrainedBlock(Ident, Ident, Vec<Ident>),
    ExternalName,
    ExternalType(Ident),
    ListenerName,
    ListenerParams(Ident),
    ListenerBlock(Ident, Vec<Param>),
    OpGenSet,
}

#[derive(Debug)]
enum Identifier {
    Dynamic(Dynamic),
    Constrained(Constrained),
    External(External),
    Listener(Listener),
}

#[derive(Debug)]
struct Dynamic {
    ty: Ident,
    dependents: BTreeSet<usize>,
}

#[derive(Debug)]
struct Param {
    name: Ident,
    is_ref: bool,
}

impl Param {
    fn parse_params(token_stream: TokenStream) -> Vec<Param> {
        let mut params = Vec::new();
        let mut is_ref = false;
        for token in token_stream {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == ',' => {}, // TODO: Remove >1 comma, no comma, and leading comma
                TokenTree::Punct(punct) if punct.as_char() == '&' => is_ref = true,
                TokenTree::Ident(name) => {
                    params.push(Param {
                        name,
                        is_ref,
                    });
                    is_ref = false;
                },
                _ => panic!("Unexpected token: {}", token)
            }
        }
        params
    }
}

#[derive(Debug)]
struct Constrained {
    ty: Ident,
    params: Vec<Ident>,
    block: TokenStream,
    compute_fn_name: Ident,
    dependents: BTreeSet<usize>,
}

#[derive(Debug)]
struct External {
    ty: Ident,
    dependents: BTreeSet<usize>,
}

#[derive(Debug)]
struct Listener {
    params: Vec<Param>,
    block: TokenStream,
}