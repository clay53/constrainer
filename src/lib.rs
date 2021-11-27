use proc_macro2::{
    Group,
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

    let mut dynamics: IndexMap<Ident, Dynamic> = IndexMap::new();
    let mut constraineds: IndexMap<Ident, Constrained> = IndexMap::new();
    let mut ops: TokenStream = TokenStream::new();

    let mut parse_state = ParseState::Key;
    for token in data.stream() {
        match parse_state {
            ParseState::Key => match token {
                TokenTree::Ident(ident) if ident.to_string().as_str() == "dynamic" => parse_state = ParseState::DynamicName,
                TokenTree::Ident(ident) if ident.to_string().as_str() == "constrained" => parse_state = ParseState::ConstrainedName,
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
                        pub fn #get_fn_name(&self) -> #ty {
                            self.#name
                        }
                    });
                    dynamics.insert(name, Dynamic {
                        ty: ty,
                        dependents: BTreeSet::new(),
                    });
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
                    let mut params_parse_state = ConstrainedParamsParseState::Key;
                    for token in group.stream() {
                        match params_parse_state {
                            ConstrainedParamsParseState::Key => match token {
                                TokenTree::Punct(punct) if punct.as_char() == ',' => {}, // TODO: Remove >1 comma, no comma, and leading comma
                                TokenTree::Ident(ident) if ident.to_string().as_str() == "dynamic" => params_parse_state = ConstrainedParamsParseState::Name(ConstrainedParamType::Dynamic),
                                TokenTree::Ident(ident) if ident.to_string().as_str() == "constrained" => params_parse_state = ConstrainedParamsParseState::Name(ConstrainedParamType::Constrained),
                                _ => panic!("Unexpected token: {}", token)
                            },
                            ConstrainedParamsParseState::Name(param_ty) => match token {
                                TokenTree::Ident(name) => params_parse_state = ConstrainedParamsParseState::Type(param_ty, name),
                                _ => panic!("Unexpected token: {}", token)
                            },
                            ConstrainedParamsParseState::Type(param_ty, name) => match token {
                                TokenTree::Ident(ty) => {
                                    params.push(ConstrainedParam {
                                        param_ty,
                                        name,
                                        ty,
                                    });
                                    params_parse_state = ConstrainedParamsParseState::Key;
                                },
                                _ => panic!("Unexpected token: {}", token)
                            }
                        }
                    }
                    parse_state = ParseState::ConstrainedBlock(name, ty, params);
                },
                _ => panic!("Unexpected token: {}", token)
            },
            ParseState::ConstrainedBlock(name, ty, params) => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    let index = constraineds.len();
                    for param in &params {
                        match param.param_ty {
                            ConstrainedParamType::Dynamic => {
                                let dependents = &mut dynamics.get_mut(&param.name).unwrap().dependents;
                                dependents.insert(index);
                            },
                            ConstrainedParamType::Constrained => {
                                let dependents = &mut constraineds.get_mut(&param.name).unwrap().dependents;
                                dependents.insert(index);
                            }
                        }
                    }

                    let get_fn_name = Ident::new(&format!("get_{}", name), Span::call_site());
                    let name_string = name.to_string();
                    ops.append_all(quote! {
                        pub fn #get_fn_name(&self) -> #ty {
                            self.#name
                        }
                    });
                    constraineds.insert(name, Constrained::new(
                        name_string,
                        ty,
                        params,
                        group.stream(),
                    ));
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
                                set_dynamics.insert(dynamics.get_index_of(&ident).unwrap(), ident);
                            },
                            _ => panic!("Unexpected token: {}", token)
                        }
                    }
                    if set_dynamics.len() == 0 {
                        todo!("opgenset needs at least one dynamic");
                    }

                    let mut set_dynamics_iter = set_dynamics.iter();
                    let mut fn_name = format!("set_{}", set_dynamics_iter.next().unwrap().1);
                    for (_, name) in set_dynamics_iter{
                        fn_name.push_str(&format!("_{}", name));
                    }
                    let fn_name = Ident::new(&fn_name, Span::call_site());

                    let mut fn_args = TokenStream::new();
                    fn_args.append_all(quote! { &mut self, });
                    for (i, name) in &set_dynamics {
                        let ty = &dynamics.get_index(*i).unwrap().1.ty;
                        fn_args.append_all(quote! {
                            #name: #ty,
                        });
                    }

                    let mut fn_block = TokenStream::new();
                    for (_, name) in &set_dynamics {
                        let name = name;
                        fn_block.append_all(quote! {
                            self.#name = #name;
                        });
                    }
                    
                    let mut to_update = BTreeSet::new();

                    for (i, _) in &set_dynamics {
                        for dependent in &dynamics.get_index(*i).unwrap().1.dependents {
                            to_update.insert(*dependent);
                        }
                    }

                    let mut last_update_size = to_update.len();
                    let mut new_updates = BTreeSet::new();
                    for i in &to_update {
                        let (_, constrained) = constraineds.get_index(*i).unwrap();
                        for dependent in &constrained.dependents {
                            new_updates.insert(*dependent);
                        }
                    }
                    to_update.append(&mut new_updates);
                    
                    while last_update_size != to_update.len() {
                        let mut new_updates = BTreeSet::new();
                        for i in &to_update {
                            for dependent in &constraineds.get_index(*i).unwrap().1.dependents {
                                new_updates.insert(*dependent);
                            }
                        }
                        last_update_size = to_update.len();
                        to_update.append(&mut new_updates);
                    }

                    for i in &to_update {
                        let constrained = constraineds.get_index(*i).unwrap();
                        let name = &constrained.0;
                        let compute_fn_name = &constrained.1.compute_fn_name;
                        let mut fn_args = TokenStream::new();
                        for param in &constrained.1.params {
                            let name = &param.name;
                            fn_args.append_all(quote! {
                                self.#name,
                            });
                        }
                        fn_block.append_all(quote! {
                            self.#name = Self::#compute_fn_name(#fn_args);
                        });
                    }

                    ops.append_all(quote! {
                        fn #fn_name(#fn_args) {
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
        struct #name
    });
    let mut fields = TokenStream::new();
    for (name, dynamic) in &dynamics {
        let name = name;
        let ty = &dynamic.ty;
        fields.append_all(quote! {
            #name: #ty,
        });
    }
    for (name, constrained) in &constraineds {
        let name = name;
        let ty = &constrained.ty;
        fields.append_all(quote! {
            #name: #ty,
        });
    }
    out.append(Group::new(Delimiter::Brace, fields));

    out.append_all(quote! { impl #name });
    let mut methods = TokenStream::new();

    methods.append_all(quote! { fn new });
    let mut new_args = TokenStream::new();
    for (name, dynamic) in &dynamics {
        let name = name;
        let ty = &dynamic.ty;
        new_args.append_all(quote! {
            #name: #ty,
        });
    }
    methods.append(Group::new(Delimiter::Parenthesis, new_args));
    methods.append_all(quote! { -> Self });
    let mut new_self = TokenStream::new();
    for (name, constrained) in &constraineds {
        let name = name;
        let compute_fn_name = &constrained.compute_fn_name;
        let mut compute_args = TokenStream::new();
        for param in &constrained.params {
            let name = &param.name;
            compute_args.append_all(quote! {
                #name,
            });
        }
        new_self.append_all(quote! {
            let #name = Self::#compute_fn_name(#compute_args);
        });
    }
    let mut new_self_fields = TokenStream::new();
    for (name, _) in &dynamics {
        let name = name;
        new_self_fields.append_all(quote! {
            #name,
        });
    }
    for (name, _) in &constraineds {
        let name = name;
        new_self_fields.append_all(quote! {
            #name,
        });
    }
    new_self.append_all(quote! {
        Self {
            #new_self_fields
        }
    });
    methods.append(Group::new(Delimiter::Brace, new_self));

    for (_, constrained) in &constraineds {
        let compute_fn_name = &constrained.compute_fn_name;
        let mut compute_args = TokenStream::new();
        for param in &constrained.params {
            let name = &param.name;
            let ty = &param.ty;
            compute_args.append_all(quote! {
                #name: #ty,
            });
        }
        let ty = &constrained.ty;
        let block = &constrained.block;
        methods.append_all(quote! { fn #compute_fn_name (#compute_args) -> #ty { #block }});
    }

    methods.append_all(ops);

    out.append(Group::new(Delimiter::Brace, methods));

    // println!("{:#}", out);

    out.into()
}

enum ParseState {
    Key,
    DynamicName,
    DynamicType(Ident),
    ConstrainedName,
    ConstrainedType(Ident),
    ConstrainedParams(Ident, Ident),
    ConstrainedBlock(Ident, Ident, Vec<ConstrainedParam>),
    OpGenSet,
}

enum ConstrainedParamsParseState {
    Key,
    Name(ConstrainedParamType),
    Type(ConstrainedParamType, Ident),
}

struct Dynamic {
    ty: Ident,
    dependents: BTreeSet<usize>,
}

#[derive(Debug)]
struct Constrained {
    ty: Ident,
    params: Vec<ConstrainedParam>,
    block: TokenStream,
    compute_fn_name: Ident,
    dependents: BTreeSet<usize>
}

impl Constrained {
    fn new(name: String, ty: Ident, params: Vec<ConstrainedParam>, block: TokenStream) -> Self {
        Self {
            ty,
            params,
            block,
            compute_fn_name: Ident::new(&format!("compute_{}", name), Span::call_site()),
            dependents: BTreeSet::new(),
        }
    }
}

#[derive(Debug)]
struct ConstrainedParam {
    param_ty: ConstrainedParamType,
    name: Ident,
    ty: Ident,
}

#[derive(Debug)]
enum ConstrainedParamType {
    Dynamic,
    Constrained,
}