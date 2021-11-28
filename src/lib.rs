use proc_macro2::{
    Ident,
    TokenStream,
    TokenTree,
    Delimiter,
    Span
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
                    for param in &params {
                        let identifier = identifiers.get_mut(param).unwrap();
                        match identifier {
                            Identifier::Dynamic(dynamic) => {
                                dynamic.dependents.insert(index);
                            },
                            Identifier::Constrained(constrained) => {
                                constrained.dependents.insert(index);
                            }
                            Identifier::External(external) => {
                                external.dependents.insert(index);
                            }
                        }
                    }

                    let get_fn_name = Ident::new(&format!("get_{}", name), Span::call_site());
                    let name_string = name.to_string();
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
                    let compute_fn_name = Ident::new(&format!("compute_{}", name_string), Span::call_site());
                    let mut compute_args = TokenStream::new();
                    let mut init_args = TokenStream::new();
                    for param in &params {
                        let ty = match identifiers.get(param).unwrap() {
                            Identifier::Dynamic(dynamic) => &dynamic.ty,
                            Identifier::Constrained(constrained) => &constrained.ty,
                            Identifier::External(external) => &external.ty,
                        };
                        compute_args.append_all(quote! {
                            #param: #ty,
                        });
                        init_args.append_all(quote! {
                            #param,
                        });
                    }
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
                    // deliminated_externals.append_all(quote! {
                    //     #name,
                    // });
                    identifiers.insert(name, Identifier::External(External {
                        ty,
                        dependents: BTreeSet::new(),
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

                    let mut fn_args = TokenStream::new();
                    let mut fn_block = TokenStream::new();
                    fn_args.append_all(quote! { &mut self, });
                    for (_, (name, dynamic)) in &set_dynamics {
                        let ty = &dynamic.ty;
                        fn_args.append_all(quote! {
                            #name: #ty,
                        });
                        fn_block.append_all(quote! {
                            self.#name = #name;
                        });
                    }
                    
                    let mut to_update = BTreeMap::new();

                    for (_, (_, dynamic)) in &set_dynamics {
                        for dependent in &dynamic.dependents {
                            let constrained = if let (name, Identifier::Constrained(constrained)) = identifiers.get_index(*dependent).unwrap() {
                                (name, constrained)
                            } else {
                                unreachable!()
                            };
                            to_update.insert(*dependent, constrained);
                        }
                    }

                    let mut last_update_size = to_update.len();
                    let mut new_updates = BTreeMap::new();
                    for (_, (_, queued)) in &to_update {
                        for dependent in &queued.dependents {
                            let constrained = if let (name, Identifier::Constrained(constrained)) = identifiers.get_index(*dependent).unwrap() {
                                (name, constrained)
                            } else {
                                unreachable!()
                            };
                            new_updates.insert(*dependent, constrained);
                        }
                    }
                    to_update.append(&mut new_updates);
                    
                    while last_update_size != to_update.len() {
                        let mut new_updates = BTreeMap::new();
                        for (_, (_, queued)) in &to_update {
                            for dependent in &queued.dependents {
                                let constrained = if let (name, Identifier::Constrained(constrained)) = identifiers.get_index(*dependent).unwrap() {
                                    (name, constrained)
                                } else {
                                    unreachable!()
                                };
                                new_updates.insert(*dependent, constrained);
                            }
                        }
                        last_update_size = to_update.len();
                        to_update.append(&mut new_updates);
                    }

                    for (_, (name, constrained)) in &to_update {
                        let compute_fn_name = &constrained.compute_fn_name;
                        let mut compute_fn_args = TokenStream::new();
                        for param in &constrained.params {
                            let name = &param;
                            match identifiers.get(param).unwrap() {
                                Identifier::Dynamic(_) | Identifier::Constrained(_) => {
                                    compute_fn_args.append_all(quote! {
                                        self.#name,
                                    });
                                },
                                Identifier::External(External {
                                    ty,
                                    ..
                                }) => {
                                    fn_args.append_all(quote! {
                                        #name: #ty,
                                    });
                                    compute_fn_args.append_all(quote! {
                                        #name
                                    });
                                }
                            }
                        }
                        fn_block.append_all(quote! {
                            self.#name = Self::#compute_fn_name(#compute_fn_args);
                        });
                    }

                    ops.append_all(quote! {
                        pub fn #fn_name(#fn_args) {
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

    println!("{:#}", out);

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
    OpGenSet,
}

#[derive(Debug)]
enum Identifier {
    Dynamic(Dynamic),
    Constrained(Constrained),
    External(External),
}

#[derive(Debug)]
struct Dynamic {
    ty: Ident,
    dependents: BTreeSet<usize>,
}

#[derive(Debug)]
struct Constrained {
    ty: Ident,
    params: Vec<Ident>,
    block: TokenStream,
    compute_fn_name: Ident,
    dependents: BTreeSet<usize>
}

#[derive(Debug)]
struct External {
    ty: Ident,
    dependents: BTreeSet<usize>,
}