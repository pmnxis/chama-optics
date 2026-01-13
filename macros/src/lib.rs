/*
 * SPDX-FileCopyrightText: © 2026 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Procedural macro for automatic ThemeParameters trait implementation

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta};

/// Derive macro for automatic ThemeParameters trait implementation
///
/// Usage:
/// ```rust
/// #[derive(ThemeParameters)]
/// struct MyTheme {
///     #[param(slider, min = 0.0, max = 500.0, label = "Border Left", default = 90)]
///     border_left: u32,
///
///     #[param(color, label = "Font Color", default = "BLACK")]
///     font_color: egui::Color32,
///
///     #[param(text, label_key = "theme.left_text", hint_key = "theme.template_hint", default_const = "DEFAULT_LEFT")]
///     left_text: String,
/// }
/// ```
#[proc_macro_derive(ThemeParameters, attributes(param))]
pub fn derive_theme_parameters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ThemeParameters can only be derived for structs with named fields"),
        },
        _ => panic!("ThemeParameters can only be derived for structs"),
    };

    let mut schema_params = Vec::new();
    let mut update_arms = Vec::new();
    let mut ui_code = Vec::new();
    let mut field_metadata = Vec::new(); // Store metadata for each field

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let _field_type = &field.ty;

        // Find #[param(...)] attribute
        let param_attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("param"));

        if param_attr.is_none() {
            continue; // Skip fields without #[param] attribute
        }

        let param_attr = param_attr.unwrap();

        // Parse the attribute arguments
        let mut param_type = String::new();
        let mut min = None;
        let mut max = None;
        let mut label = String::new();
        let mut hint = None;
        let mut default = String::new();
        let mut field_path = None;
        let mut label_key = None;
        let mut hint_key = None;
        let mut default_const = None;
        let mut default_border = None;
        let mut default_limit = None;

        if let Meta::List(meta_list) = &param_attr.meta {
            let tokens = &meta_list.tokens;
            let parsed: syn::Result<ParamArgs> = syn::parse2(tokens.clone());

            match parsed {
                Ok(args) => {
                    param_type = args.param_type;
                    min = args.min;
                    max = args.max;
                    label = args.label;
                    hint = args.hint;
                    default = args.default;
                    field_path = args.field_path;
                    label_key = args.label_key;
                    hint_key = args.hint_key;
                    default_const = args.default_const;
                    default_border = args.default_border;
                    default_limit = args.default_limit;
                }
                Err(e) => {
                    return syn::Error::new_spanned(
                        param_attr,
                        format!("Failed to parse param attribute: {}", e),
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }

        let field_name_str = field_name.to_string();
        let param_key = field_name_str.replace('_', ".");

        // Determine the field access path
        let field_access = if let Some(ref path) = field_path {
            // Use custom field path from attribute - build it manually
            let mut tokens = quote! { self };
            for part in path.split('.') {
                let ident = syn::Ident::new(part, field_name.span());
                tokens = quote! { #tokens.#ident };
            }
            tokens
        } else {
            // Default: use field name
            quote! { self.#field_name }
        };

        // Generate schema parameter based on type
        match param_type.as_str() {
            "border" => {
                // Special type for Border struct - generates 5 parameters (left, right, top, bottom, color)
                let default_border_name = default_border
                    .as_ref()
                    .expect("border requires 'default_border' attribute");
                let default_limit_name = default_limit
                    .as_ref()
                    .expect("border requires 'default_limit' attribute");

                let default_border_ident = syn::Ident::new(default_border_name, field_name.span());
                let default_limit_ident = syn::Ident::new(default_limit_name, field_name.span());

                // Use label_key for i18n label generation
                let base_label_key = label_key
                    .as_ref()
                    .expect("border requires 'label_key' attribute (e.g., 'theme.border')");

                // Generate 4 slider parameters for border sides
                for side in &["left", "right", "top", "bottom"] {
                    let side_key = format!("{}.{}", param_key, side);
                    let side_label_key = format!("{}.{}", base_label_key, side);
                    let side_field = syn::Ident::new(side, field_name.span());

                    schema_params.push(quote! {
                        crate::param_slider!(
                            #side_key,
                            rust_i18n::t!(#side_label_key),
                            #default_limit_ident.#side_field.0 as f64,
                            #default_limit_ident.#side_field.1 as f64,
                            #default_border_ident.#side_field,
                            #field_access.#side_field
                        )
                    });

                    update_arms.push(quote! {
                        #side_key => (#field_access.#side_field, u32)
                    });
                }

                // Generate 1 color parameter for border color
                let color_key = format!("{}.color", param_key);
                let color_label_key = format!("{}.color", base_label_key);
                let color_field = syn::Ident::new("color", field_name.span());

                schema_params.push(quote! {
                    crate::param_color!(
                        #color_key,
                        rust_i18n::t!(#color_label_key),
                        #default_border_ident.#color_field,
                        #field_access.#color_field
                    )
                });

                update_arms.push(quote! {
                    #color_key => (#field_access.#color_field, color)
                });

                // Generate UI for border - call ui_config on the Border struct
                field_metadata.push((
                    "border",
                    field_name.clone(),
                    default_border_name.clone(),
                    default_limit_name.clone(),
                ));
            }
            "slider" => {
                let min_val = min.expect("slider requires 'min' attribute");
                let max_val = max.expect("slider requires 'max' attribute");

                // Use default_const if provided, otherwise use default
                let default_val = if let Some(ref const_name) = default_const {
                    let ident = syn::Ident::new(const_name, field_name.span());
                    quote! { #ident }
                } else {
                    let def_str = default.clone();
                    quote! { #def_str }
                };

                // Use label_key to generate t!() or use literal label
                let label_expr = if let Some(ref key) = label_key {
                    quote! { rust_i18n::t!(#key) }
                } else {
                    quote! { #label }
                };

                // Use hint_key to generate t!() or use literal hint
                let hint_expr = if let Some(ref key) = hint_key {
                    Some(quote! { rust_i18n::t!(#key) })
                } else {
                    hint.as_ref().map(|h| quote! { #h })
                };

                if let Some(ref hint_tokens) = hint_expr {
                    schema_params.push(quote! {
                        crate::param_slider!(
                            #param_key,
                            #label_expr,
                            #hint_tokens,
                            #min_val,
                            #max_val,
                            #default_val,
                            #field_access
                        )
                    });
                } else {
                    schema_params.push(quote! {
                        crate::param_slider!(
                            #param_key,
                            #label_expr,
                            #min_val,
                            #max_val,
                            #default_val,
                            #field_access
                        )
                    });
                }

                update_arms.push(quote! {
                    #param_key => (#field_access, u32)
                });

                // Generate UI for slider
                let min_range = min_val as u32;
                let max_range = max_val as u32;

                // Get default ident from default_const or default
                let default_ident_name = default_const.clone().unwrap_or(default.clone());
                let default_ident = syn::Ident::new(&default_ident_name, field_name.span());

                if let Some(ref hint_tokens) = hint_expr {
                    ui_code.push(quote! {
                        crate::ui_slider!(ui, #field_access, #default_ident, #min_range..=#max_range, #label_expr, #hint_tokens);
                    });
                } else {
                    ui_code.push(quote! {
                        crate::ui_slider!(ui, #field_access, #default_ident, #min_range..=#max_range, #label_expr);
                    });
                }
            }
            "color" => {
                // Parse default color value
                let default_color = if let Some(ref const_name) = default_const {
                    let ident = syn::Ident::new(const_name, field_name.span());
                    quote! { #ident }
                } else {
                    // Support both named colors and RGB values
                    match default.as_str() {
                        "BLACK" => quote! { egui::Color32::BLACK },
                        "WHITE" => quote! { egui::Color32::WHITE },
                        "RED" => quote! { egui::Color32::RED },
                        "GREEN" => quote! { egui::Color32::GREEN },
                        "BLUE" => quote! { egui::Color32::BLUE },
                        "TRANSPARENT" => quote! { egui::Color32::TRANSPARENT },
                        // Support hex format: "#RRGGBB" or "RRGGBB"
                        s if s.starts_with('#') || s.len() == 6 => {
                            let hex = s.trim_start_matches('#');
                            if hex.len() == 6 {
                                if let (Ok(r), Ok(g), Ok(b)) = (
                                    u8::from_str_radix(&hex[0..2], 16),
                                    u8::from_str_radix(&hex[2..4], 16),
                                    u8::from_str_radix(&hex[4..6], 16),
                                ) {
                                    quote! { egui::Color32::from_rgb(#r, #g, #b) }
                                } else {
                                    quote! { egui::Color32::BLACK }
                                }
                            } else {
                                quote! { egui::Color32::BLACK }
                            }
                        }
                        // Support "rgb(r, g, b)" format
                        s if s.starts_with("rgb(") && s.ends_with(')') => {
                            let rgb = &s[4..s.len() - 1];
                            let parts: Vec<&str> = rgb.split(',').map(|s| s.trim()).collect();
                            if parts.len() == 3 {
                                if let (Ok(r), Ok(g), Ok(b)) = (
                                    parts[0].parse::<u8>(),
                                    parts[1].parse::<u8>(),
                                    parts[2].parse::<u8>(),
                                ) {
                                    quote! { egui::Color32::from_rgb(#r, #g, #b) }
                                } else {
                                    quote! { egui::Color32::BLACK }
                                }
                            } else {
                                quote! { egui::Color32::BLACK }
                            }
                        }
                        _ => quote! { egui::Color32::BLACK },
                    }
                };

                // Use label_key to generate t!() or use literal label
                let label_expr = if let Some(ref key) = label_key {
                    quote! { rust_i18n::t!(#key) }
                } else {
                    quote! { #label }
                };

                schema_params.push(quote! {
                    crate::param_color!(
                        #param_key,
                        #label_expr,
                        #default_color,
                        #field_access
                    )
                });

                update_arms.push(quote! {
                    #param_key => (#field_access, color)
                });

                // Generate UI for color picker
                ui_code.push(quote! {
                    crate::ui_color!(ui, #field_access, #label_expr);
                });
            }
            "text" => {
                // Use label_key to generate t!() or use literal label
                let label_expr = if let Some(ref key) = label_key {
                    quote! { rust_i18n::t!(#key) }
                } else {
                    quote! { #label }
                };

                // Use hint_key to generate t!() or use literal hint
                let hint_expr = if let Some(ref key) = hint_key {
                    quote! { rust_i18n::t!(#key) }
                } else if let Some(ref h) = hint {
                    quote! { #h }
                } else {
                    return syn::Error::new_spanned(
                        param_attr,
                        "text requires 'hint' or 'hint_key' attribute",
                    )
                    .to_compile_error()
                    .into();
                };

                // Use default_const if provided
                let default_val = if let Some(ref const_name) = default_const {
                    let ident = syn::Ident::new(const_name, field_name.span());
                    quote! { #ident.text }
                } else {
                    quote! { #default }
                };

                // For text fields, we need to access .text subfield
                let text_field_access = quote! { #field_access.text };

                schema_params.push(quote! {
                    crate::param_text!(
                        #param_key,
                        #label_expr,
                        #hint_expr,
                        #default_val,
                        #text_field_access
                    )
                });

                update_arms.push(quote! {
                    #param_key => (#text_field_access, string)
                });

                // Generate UI for text - use VariableTextSlot::ui() method
                let default_ident_name = default_const.clone().unwrap_or(default.clone());
                let default_ident = syn::Ident::new(&default_ident_name, field_name.span());
                ui_code.push(quote! {
                    #field_access.ui(ui, #label_expr, &#default_ident);
                    ui.end_row();
                });
            }
            "font" => {
                // Use label_key to generate t!() or use literal label
                let label_expr = if let Some(ref key) = label_key {
                    quote! { rust_i18n::t!(#key) }
                } else {
                    quote! { #label }
                };

                // Use hint_key to generate t!() or use literal hint
                let hint_expr = if let Some(ref key) = hint_key {
                    quote! { rust_i18n::t!(#key) }
                } else if let Some(ref h) = hint {
                    quote! { #h }
                } else {
                    return syn::Error::new_spanned(
                        param_attr,
                        "font requires 'hint' or 'hint_key' attribute",
                    )
                    .to_compile_error()
                    .into();
                };

                // Use default_const if provided
                let default_val = if let Some(ref const_name) = default_const {
                    let ident = syn::Ident::new(const_name, field_name.span());
                    quote! { #ident }
                } else {
                    quote! { #default }
                };

                schema_params.push(quote! {
                    crate::param_font!(
                        #param_key,
                        #label_expr,
                        #hint_expr,
                        #default_val,
                        #field_access
                    )
                });

                update_arms.push(quote! {
                    #param_key => (#field_access, string)
                });

                // No UI for Rust - Swift will handle font selection
            }
            _ => {
                return syn::Error::new_spanned(
                    param_attr,
                    format!("Unknown parameter type: {}", param_type),
                )
                .to_compile_error()
                .into();
            }
        }
    }

    // Generate UI code for border if present
    let border_ui = if field_metadata.iter().any(|(t, _, _, _)| *t == "border") {
        let (_, border_field, default_border_name, default_limit_name) = field_metadata
            .iter()
            .find(|(t, _, _, _)| *t == "border")
            .unwrap();
        let default_border_ident = syn::Ident::new(default_border_name, border_field.span());
        let default_limit_ident = syn::Ident::new(default_limit_name, border_field.span());
        quote! {
            self.#border_field.ui_config(ui, &#default_border_ident, &#default_limit_ident);
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl crate::theme::parameter_schema::ThemeParameters for #struct_name {
            fn schema(&self) -> crate::theme::parameter_schema::ThemeSchema {
                crate::theme::parameter_schema::ThemeSchema {
                    theme_name: self.unique_name().to_string(),
                    theme_label: self.label().to_string(),
                    parameters: vec![
                        #(#schema_params),*
                    ],
                }
            }

            fn update_from_json(
                &mut self,
                updates: &serde_json::Map<String, serde_json::Value>
            ) -> Result<(), String> {
                crate::update_param!(updates, {
                    #(#update_arms),*
                })
            }
        }

        // Auto-generated helper methods for Theme trait
        impl #struct_name {
            /// Auto-generated get_parameters_json() implementation
            /// Call this from your Theme::get_parameters_json() implementation
            pub fn auto_get_parameters_json(&self) -> String {
                let schema = <Self as crate::theme::parameter_schema::ThemeParameters>::schema(self);
                serde_json::to_string(&schema).unwrap_or_else(|e| {
                    log::error!("Failed to serialize {} schema: {}", stringify!(#struct_name), e);
                    r#"{"error": "serialization failed"}"#.to_string()
                })
            }

            /// Auto-generated ui_config() implementation
            /// Call this from your Theme::ui_config() implementation
            pub fn auto_ui_config(&mut self, ui: &mut egui::Ui) {
                #border_ui

                ui.vertical(|ui| {
                    ui.add_space(4.0);

                    egui::Grid::new(concat!(stringify!(#struct_name), "_config_grid"))
                        .num_columns(2)
                        .spacing([4.0, 3.0])
                        .show(ui, |ui| {
                            #(#ui_code)*
                        });
                });
            }
        }
    };

    TokenStream::from(expanded)
}

// Helper struct for parsing #[param(...)] attributes
struct ParamArgs {
    param_type: String,
    min: Option<f64>,
    max: Option<f64>,
    label: String,
    hint: Option<String>,
    default: String,
    field_path: Option<String>,
    // New: i18n key support
    label_key: Option<String>,
    hint_key: Option<String>,
    default_const: Option<String>,
    default_border: Option<String>,
    default_limit: Option<String>,
}

impl syn::parse::Parse for ParamArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut min = None;
        let mut max = None;
        let mut label = String::new();
        let mut hint = None;
        let mut default = String::new();
        let field_path = None;
        let mut label_key = None;
        let mut hint_key = None;
        let mut default_const = None;
        let mut default_border = None;
        let mut default_limit = None;

        // First token should be the parameter type (slider, color, text)
        let type_ident: syn::Ident = input.parse()?;
        let param_type = type_ident.to_string();

        // Parse remaining key-value pairs
        while !input.is_empty() {
            input.parse::<syn::Token![,]>()?;

            if input.is_empty() {
                break;
            }

            let key: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;

            match key.to_string().as_str() {
                "min" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Float(f) = lit {
                        min = Some(f.base10_parse()?);
                    } else if let Lit::Int(i) = lit {
                        min = Some(i.base10_parse::<f64>()?);
                    }
                }
                "max" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Float(f) = lit {
                        max = Some(f.base10_parse()?);
                    } else if let Lit::Int(i) = lit {
                        max = Some(i.base10_parse::<f64>()?);
                    }
                }
                "label" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        label = s.value();
                    }
                }
                "hint" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        hint = Some(s.value());
                    }
                }
                "default" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        default = s.value();
                    } else if let Lit::Int(i) = lit {
                        default = i.base10_digits().to_string();
                    } else if let Lit::Float(f) = lit {
                        default = f.base10_digits().to_string();
                    }
                }
                "label_key" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        label_key = Some(s.value());
                    }
                }
                "hint_key" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        hint_key = Some(s.value());
                    }
                }
                "default_const" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        default_const = Some(s.value());
                    }
                }
                "default_border" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        default_border = Some(s.value());
                    }
                }
                "default_limit" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        default_limit = Some(s.value());
                    }
                }
                _ => {}
            }
        }

        Ok(ParamArgs {
            param_type,
            min,
            max,
            label,
            hint,
            default,
            field_path,
            label_key,
            hint_key,
            default_const,
            default_border,
            default_limit,
        })
    }
}
