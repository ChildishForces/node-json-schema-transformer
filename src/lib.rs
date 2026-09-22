#![deny(clippy::all)]

use std::collections::HashMap;

use json_schema_transformer as jst;
use jst::emit::{EmitOptions, Emitter, SharedHelpers};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;

/// Target language/framework for generated output.
#[napi(string_enum = "lowercase")]
pub enum Language {
  Zod,
  TypeScript,
  Pydantic,
  Swift,
  Kotlin,
  Rust,
}

fn emitter_for(language: Language) -> &'static dyn Emitter {
  match language {
    Language::Zod => &jst::ZodEmitter,
    Language::TypeScript => &jst::TypeScriptEmitter,
    Language::Pydantic => &jst::PydanticEmitter,
    Language::Swift => &jst::SwiftEmitter,
    Language::Kotlin => &jst::KotlinEmitter,
    Language::Rust => &jst::RustEmitter,
  }
}

fn convert_error(e: jst::ConvertError) -> Error {
  Error::from_reason(e.to_string())
}

/// Explicit name first, then the schema's root `title`.
fn resolve_name(schema: &Value, name: Option<&str>) -> Result<String> {
  if let Some(n) = name {
    return Ok(n.to_string());
  }
  schema
    .as_object()
    .and_then(|o| o.get("title"))
    .and_then(|t| t.as_str())
    .map(|t| t.to_string())
    .ok_or_else(|| convert_error(jst::ConvertError::MissingName))
}

fn convert_schema(
  schema: &Value,
  remotes: Option<HashMap<String, Value>>,
  enforce_formats: Option<bool>,
) -> Result<jst::ConvertedSchema> {
  jst::Converter::convert_with_options(
    schema,
    &remotes.unwrap_or_default(),
    enforce_formats.unwrap_or(true),
  )
  .map_err(convert_error)
}

/// Location of the shared helpers file relative to a generated module
/// (collection mode). Without this, helpers are inlined and the module is
/// self-contained.
#[napi(object)]
#[derive(Clone)]
pub struct SharedHelpersOptions {
  /// Helper file name at the collection root, e.g. "jst-helpers.ts".
  pub file_name: String,
  /// Relative path prefix from the generated file's directory back to the
  /// collection root: "" (default) for root-level files, one "../" per
  /// nesting level.
  pub dir_prefix: Option<String>,
}

impl From<SharedHelpersOptions> for SharedHelpers {
  fn from(opts: SharedHelpersOptions) -> Self {
    let helpers = SharedHelpers::new(opts.file_name);
    match opts.dir_prefix {
      Some(prefix) => helpers.with_dir_prefix(prefix),
      None => helpers,
    }
  }
}

#[napi(object)]
#[derive(Default)]
pub struct TransformOptions {
  /// Generated root type name (PascalCased). Falls back to the schema's root
  /// `title`; errors when neither is available.
  pub name: Option<String>,
  /// Emit mutable stored properties (Swift `var`, Kotlin `var`; Pydantic
  /// enables assignment re-validation). No-op for Rust and TypeScript/Zod.
  pub mutable: Option<bool>,
  /// Collection mode: reference helpers from a shared companion file instead
  /// of inlining them. Prefer `CollectionSession` which also tailors the
  /// helpers file to what the emitted modules actually need.
  pub helpers: Option<SharedHelpersOptions>,
  /// Remote schema documents (URI → document) that non-fragment `$ref`s may
  /// resolve against.
  pub remotes: Option<HashMap<String, Value>>,
  /// Enforce `format` during validation (default true). Draft 2020-12 treats
  /// `format` as annotation-only; set false for that behavior.
  pub enforce_formats: Option<bool>,
}

/// Convert a JSON Schema to generated code for the given target language.
#[napi]
pub fn transform(
  schema: Value,
  language: Language,
  options: Option<TransformOptions>,
) -> Result<String> {
  let opts = options.unwrap_or_default();
  let name = resolve_name(&schema, opts.name.as_deref())?;
  let converted = convert_schema(&schema, opts.remotes, opts.enforce_formats)?;
  let emit_options = EmitOptions::new()
    .with_mutable(opts.mutable.unwrap_or(false))
    .with_helpers(opts.helpers.map(SharedHelpers::from));
  Ok(emitter_for(language).emit_with_options(&converted, &name, &emit_options))
}

/// Convert a JSON Schema to a complete Zod TypeScript module string.
#[napi]
pub fn json_schema_to_zod_module(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_zod_module(&schema, name.as_deref()).map_err(convert_error)
}

/// Convert a JSON Schema to TypeScript type definitions (.d.ts).
#[napi]
pub fn json_schema_to_typescript(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_typescript(&schema, name.as_deref()).map_err(convert_error)
}

/// Convert a JSON Schema to a Python Pydantic model.
#[napi]
pub fn json_schema_to_pydantic(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_pydantic(&schema, name.as_deref()).map_err(convert_error)
}

/// Convert a JSON Schema to a Swift Codable struct.
#[napi]
pub fn json_schema_to_swift(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_swift(&schema, name.as_deref()).map_err(convert_error)
}

/// Convert a JSON Schema to a Kotlin data class.
#[napi]
pub fn json_schema_to_kotlin(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_kotlin(&schema, name.as_deref()).map_err(convert_error)
}

/// Convert a JSON Schema to Rust serde types with validating deserialization.
#[napi]
pub fn json_schema_to_rust(schema: Value, name: Option<String>) -> Result<String> {
  jst::json_schema_to_rust(&schema, name.as_deref()).map_err(convert_error)
}

/// File extension for the language's generated output (e.g. "zod.ts", "d.ts", "py").
#[napi]
pub fn extension_for(language: Language) -> String {
  emitter_for(language).extension().to_string()
}

/// Conventional shared helpers file name for the language, or null for
/// languages that emit no runtime helpers.
#[napi]
pub fn default_helpers_file(language: Language) -> Option<String> {
  emitter_for(language)
    .default_helpers_file()
    .map(str::to_string)
}

/// Full content of the language's shared helpers file (a superset of every
/// possible need), or null when the language emits no runtime helpers. Prefer
/// `CollectionSession#helpers` which tailors the file to actual needs.
#[napi]
pub fn helpers_content(language: Language) -> Option<String> {
  emitter_for(language).helpers_content()
}

#[napi(object)]
#[derive(Default)]
pub struct CollectionSessionOptions {
  /// Custom helpers file name; defaults to the language's conventional name.
  pub helpers_file: Option<String>,
  /// Emit mutable stored properties (see `TransformOptions#mutable`).
  pub mutable: Option<bool>,
}

#[napi(object)]
#[derive(Default)]
pub struct ConvertOptions {
  /// Remote schema documents (URI → document) that non-fragment `$ref`s may
  /// resolve against.
  pub remotes: Option<HashMap<String, Value>>,
  /// Enforce `format` during validation (default true).
  pub enforce_formats: Option<bool>,
}

/// The shared helpers companion file for an emitted collection.
#[napi(object)]
pub struct HelpersFile {
  pub file_name: String,
  pub content: String,
}

/// Emits a collection of schemas that share one tailored helpers file.
///
/// Each `emit` call produces one module in collection mode (helpers
/// referenced, never inlined) and accumulates the helper components that
/// module needs. After all members are emitted, `helpers()` yields the shared
/// helpers file containing exactly the union of accumulated needs — or null
/// when no member needed any helpers.
#[napi]
pub struct CollectionSession {
  inner: jst::CollectionSession<'static>,
}

#[napi]
impl CollectionSession {
  #[napi(constructor)]
  pub fn new(language: Language, options: Option<CollectionSessionOptions>) -> Self {
    let opts = options.unwrap_or_default();
    let emitter = emitter_for(language);
    let inner = match opts.helpers_file {
      Some(file_name) => jst::CollectionSession::with_helpers_file(emitter, file_name),
      None => jst::CollectionSession::new(emitter),
    };
    Self {
      inner: inner.mutable(opts.mutable.unwrap_or(false)),
    }
  }

  /// Emit one member module. `dirPrefix` is the relative path from the
  /// module's output directory back to the collection root: "" (default) for
  /// root-level files, one "../" per nesting level.
  #[napi]
  pub fn emit(
    &mut self,
    schema: Value,
    name: Option<String>,
    dir_prefix: Option<String>,
    options: Option<ConvertOptions>,
  ) -> Result<String> {
    let opts = options.unwrap_or_default();
    let resolved = resolve_name(&schema, name.as_deref())?;
    let converted = convert_schema(&schema, opts.remotes, opts.enforce_formats)?;
    Ok(
      self
        .inner
        .emit_converted(&converted, &resolved, dir_prefix.as_deref().unwrap_or("")),
    )
  }

  /// The shared helpers file name this session will use, or null for
  /// languages that emit no runtime helpers.
  #[napi]
  pub fn helpers_file_name(&self) -> Option<String> {
    self.inner.helpers_file_name().map(str::to_string)
  }

  /// Helper component keys needed by all members emitted so far.
  #[napi]
  pub fn helper_needs(&self) -> Vec<String> {
    self
      .inner
      .helper_needs()
      .0
      .iter()
      .map(|s| s.to_string())
      .collect()
  }

  /// The tailored shared helpers file, or null when no member needed helpers
  /// (no file should be written).
  #[napi]
  pub fn helpers(&self) -> Option<HelpersFile> {
    self
      .inner
      .helpers()
      .map(|(file_name, content)| HelpersFile { file_name, content })
  }
}
