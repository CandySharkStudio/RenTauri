// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod aeskey;
mod decrypt;
mod lua_reader;
mod parser;
mod util;

use std::collections::BTreeMap;

use aeskey::AES_KEY;
use decrypt::decrypt_to_memory;
use parser::parse_embed;
use util::*;

#[derive(serde::Deserialize, Clone)]
#[serde(untagged)]
enum ErrorCode {
    CannotGetExecuteDir = -1,
    CannotReadGamesPath = -2,
    CannotCreateGameDir = -3,
    CannotDecryptData = -4,
    CannotParseLuaFile = -5,
    CannotCreateLuaGlobal = -6,
    CannotFindSaveDirectoryDefine = -7,
    CannotInitHomeDir = -10000,
    AesKeyHasWrong = -2147483647,
    NotImplements = -2147483648,
}
// 自主实现一个 serde::Serialize，以便于将 Enum 值转换成 i32，便于更好的排除一些 bug。
impl serde::Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.clone() as i32)
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct RTError {
    msg: String,
    code: ErrorCode,
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct CopywritingStruct {
    // 全局定义
    define: BTreeMap<String, serde_json::Value>,
    // 部分二进制资源。
    resource: BTreeMap<String, String>,
    // 当前翻译
    translate: BTreeMap<String, BTreeMap<String, String>>,
    // 当前样式表
    style: BTreeMap<String, String>,
    // 最终的文案代码！
    copywriting: serde_json::Value,
}
pub static COPY_WRITING: std::sync::OnceLock<CopywritingStruct> = std::sync::OnceLock::new();
///
/// 遍历当前 serde_json::Value，将当前 serde_json::Value 转成 LuaTable 格式。
///
pub fn json_to_lua_value(
    lua: &mlua::prelude::Lua,
    val: &serde_json::Value,
) -> Result<mlua::Value, mlua::Error> {
    match val {
        serde_json::Value::Null => Ok(mlua::Value::String(lua.create_string("null")?)),
        serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(mlua::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(mlua::Value::Number(f))
            } else {
                Err(mlua::Error::runtime("无效的 JSON 数字"))
            }
        }
        serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                let lua_val = json_to_lua_value(lua, v)?;
                table.set(i + 1, lua_val)?; // 特殊：LuaTable 索引从 1 开始。。
            }
            Ok(mlua::Value::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj.iter() {
                let lua_key = lua.create_string(k)?;
                let lua_val = json_to_lua_value(lua, v)?;
                table.set(lua_key, lua_val)?;
            }
            Ok(mlua::Value::Table(table))
        }
    }
}
///
/// 遍历当前 LuaTable，将当前 LuaTable 转成 serde_json 格式。
///
fn recursion_lua_table_to_json(
    key: String,
    table: &mlua::Table,
) -> Result<serde_json::Value, mlua::Error> {
    let mut r_map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut r_vec: Vec<serde_json::Value> = Vec::new();
    let mut is_object = false;
    let mut is_array = false;
    for pair in table.pairs::<mlua::Value, mlua::Value>() {
        let (k, v) = pair?;
        match k {
            mlua::Value::Integer(i) => {
                if i == (r_vec.len() as i64 + 1) {
                    r_vec.push(lua_value_to_json(i.to_string(), v)?);
                    is_array = true
                } else {
                    return Err(mlua::Error::runtime(format!(
                        "not valid num key! key: {}",
                        key
                    )));
                }
            }
            mlua::Value::String(s) => {
                is_object = true;
                let key = s.to_str()?.to_string();
                r_map.insert(s.to_str()?.to_string(), lua_value_to_json(key, v)?);
            }
            _ => {
                return Err(mlua::Error::runtime(format!(
                    "not valid num type! key: {}",
                    key
                )));
            }
        }
    }
    if is_object && is_array {
        return Err(mlua::Error::runtime(format!(
            "object and array is mixed! key: {}",
            key
        )));
    }
    if is_array {
        Ok(serde_json::Value::Array(r_vec))
    } else {
        Ok(serde_json::Value::Object(r_map))
    }
}
///
/// Lua 值转换成 JSON
///
fn lua_value_to_json(key: String, value: mlua::Value) -> Result<serde_json::Value, mlua::Error> {
    match value {
        mlua::Value::String(s) => {
            let str = s
                .to_str()
                .map_err(|_| {
                    mlua::Error::runtime(format!("Cannot convert Define to string! key: {}", key))
                })?
                .to_string();
            if str == "null" || str == "nil" {
                return Ok(serde_json::Value::Null);
            }
            Ok(serde_json::Value::String(str))
        }
        mlua::Value::Number(n) => {
            let num = serde_json::Number::from_f64(n).ok_or(mlua::Error::runtime(format!(
                "Cannot convert Define to f64! key: {}",
                key
            )))?;
            Ok(serde_json::Value::Number(num))
        }
        mlua::Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        mlua::Value::Integer(i) => Ok(i.into()),
        mlua::Value::Table(t) => Ok(recursion_lua_table_to_json(key, &t)?),
        _ => {
            return Err(mlua::Error::runtime(format!(
                "Cannot convert Define to a valid value! key: {}",
                key
            )));
        }
    }
}
///
/// 将小驼峰转换成短横线
///
fn camel_to_kebab(s: String) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

fn style_to_string(style: &mlua::Table) -> Result<String, mlua::Error> {
    let mut result = String::new();
    for pairs in style.pairs::<mlua::Value, mlua::Value>() {
        let (k, v) = pairs?;
        let k = k.to_string()?.to_string();
        let v = v.to_string()?.to_string();
        result.push_str(&format!("{}:{};", camel_to_kebab(k), v));
    }
    Ok(result)
}
///
/// 解析 Lua 文件并正确转换成文案代码！
///
#[tauri::command]
fn init_copywriting(
    app_handle: tauri::AppHandle,
    file_name: String,
) -> Result<CopywritingStruct, RTError> {
    use base64::Engine;
    let execute_dir = get_executable_file_path().ok_or(RTError {
        code: ErrorCode::CannotGetExecuteDir,
        msg: "Cannot get executable file path!".to_string(),
    })?;
    let mut engine_path = std::path::PathBuf::new();
    engine_path.push(execute_dir);
    engine_path.push("games");
    engine_path.push(file_name);
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(AES_KEY)
        .map_err(|e| RTError {
            code: ErrorCode::AesKeyHasWrong,
            msg: format!(
                "AES Key is fault! please enter a real AES Key!\nmessage: {}",
                e
            ),
        })?;
    if key_bytes.len() != 32 {
        return Err(RTError {
            code: ErrorCode::AesKeyHasWrong,
            msg: "AES Key is fault! please enter a real AES Key!\nmessage: wrong key length"
                .to_string(),
        });
    }
    let key_slice: [u8; 32] = key_bytes.try_into().unwrap();
    let engine_path = engine_path.to_string_lossy().to_string();
    let (file_map, main_lua_file_name) =
        decrypt_to_memory(engine_path, key_slice).map_err(|e| RTError {
            code: ErrorCode::CannotDecryptData,
            msg: format!(
                "Cannot decrypt your data, might be the aes key not equal the decrypt data!\nmessage: {}",
                e
            ),
        })?;
    let Some(main_lua_file_name) = main_lua_file_name else {
        return Err(RTError {
            code: ErrorCode::CannotDecryptData,
            msg: "Cannot decrypt your data, might be the aes key not equal the decrypt data!"
                .to_string(),
        });
    };
    let main_lua_content =
        String::from_utf8(file_map[&main_lua_file_name].clone()).map_err(|e| RTError {
            code: ErrorCode::CannotParseLuaFile,
            msg: format!(
                "Your lua file have a wrong encoding! please try again!\nmessage: {}",
                e
            ),
        })?;
    let real_main_lua_content = parse_embed(
        main_lua_file_name.as_str(),
        main_lua_content.as_str(),
        &file_map.clone(),
    );
    let copywriting_struct: std::rc::Rc<std::cell::RefCell<CopywritingStruct>> =
        std::rc::Rc::new(std::cell::RefCell::new(CopywritingStruct {
            define: BTreeMap::new(),
            resource: BTreeMap::new(),
            translate: BTreeMap::new(),
            style: BTreeMap::new(),
            copywriting: serde_json::Value::Null,
        }));
    use mlua::prelude::*;
    let lua = Lua::new();
    {
        let set_define_borrow = copywriting_struct.clone();
        let set_define = lua
            .create_function(move |_: &Lua, (key, value): (String, LuaValue)| {
                set_define_borrow
                    .borrow_mut()
                    .define
                    .insert(key.clone(), lua_value_to_json(key.clone(), value)?);
                Ok(())
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by set define!\nmessage: {}", e),
            })?;
        let set_translate_borrow = copywriting_struct.clone();
        let set_translate = lua
            .create_function(move |_: &Lua, (key, value): (String, LuaTable)| {
                let translate_kvargs = &mut set_translate_borrow.borrow_mut().translate;
                if !translate_kvargs.contains_key(key.as_str()) {
                    translate_kvargs.insert(key.clone(), BTreeMap::new());
                }
                let translate_kvargs = translate_kvargs.get_mut(key.as_str()).unwrap();
                for pair in value.pairs::<mlua::Value, mlua::Value>() {
                    let (k, v) = pair?;
                    let k = k.to_string()?.to_string();
                    let v = v.to_string()?.to_string();
                    translate_kvargs.insert(k, v);
                }
                Ok(())
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by set translate!\nmessage: {}", e),
            })?;
        let set_style_borrow = copywriting_struct.clone();
        let set_style = lua
            .create_function(
                move |_: &Lua, (key, value): (String, LuaValue)| match value {
                    mlua::Value::String(s) => {
                        set_style_borrow
                            .borrow_mut()
                            .style
                            .insert(key, s.to_string_lossy().to_string());
                        Ok(())
                    }
                    mlua::Value::Table(t) => {
                        set_style_borrow
                            .borrow_mut()
                            .style
                            .insert(key, style_to_string(&t)?);
                        Ok(())
                    }
                    _ => Err(mlua::Error::RuntimeError(format!(
                        "Cannot convert Define a valid value! key: {}",
                        key
                    ))),
                },
            )
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by set style!\nmessage: {}", e),
            })?;
        let get_define_borrow = copywriting_struct.clone();
        let get_define = lua
            .create_function(move |lua: &Lua, key: String| {
                let define_properties = get_define_borrow
                    .borrow()
                    .define
                    .get(key.as_str())
                    .cloned()
                    .ok_or(mlua::Error::runtime(format!(
                        "Cannot find define value by {}",
                        key
                    )))?;
                json_to_lua_value(lua, &define_properties)
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by get define!\nmessage: {}", e),
            })?;
        let get_translate = lua
            .create_function(|_: &Lua, key: String| {
                Ok(format!("<g-translate>{}</g-translate>", key))
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by get translate!\nmessage: {}", e),
            })?;
        let get_style = lua
            .create_function(|_: &Lua, key: String| Ok(format!("<g-style>{}</g-style>", key)))
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by get style!\nmessage: {}", e),
            })?;
        let get_resource_borrow = copywriting_struct.clone();
        let get_resource = lua
            .create_function(move |_: &Lua, (key, img_type): (String, String)| {
                let resource = file_map
                    .get(&key)
                    .cloned()
                    .ok_or(mlua::Error::runtime(format!(
                        "Cannot find resource by key: {}",
                        key
                    )))?;
                let base64_resource = base64::engine::general_purpose::STANDARD.encode(&resource);
                get_resource_borrow.borrow_mut().resource.insert(
                    key.clone(),
                    format!("data:{};base64,{}", img_type, base64_resource),
                );
                Ok(format!("<g-image>{}</g-image>", key.clone()))
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by Resource!\nmessage: {}", e),
            })?;
        let get_base64_resource_borrow = copywriting_struct.clone();
        let get_base64_resource = lua
            .create_function(move |_: &Lua, resource: String| {
                let rand_uuid = gen_random_uuid();
                get_base64_resource_borrow
                    .borrow_mut()
                    .resource
                    .insert(rand_uuid.clone(), format!("{}", resource));
                Ok(format!("<g-image>{}</g-image>", rand_uuid))
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!(
                    "Cannot create lua global by base64 Resource!\nmessage: {}",
                    e
                ),
            })?;
        let span = lua
            .create_function(|_: &Lua, (text, styles): (String, LuaTable)| {
                Ok(format!(
                    "<span style=\"{}\">{}</span>",
                    style_to_string(&styles)?,
                    text
                ))
            })
            .map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot create lua global by Span!\nmessage: {}", e),
            })?;
        let functions: Vec<(&str, mlua::Function)> = vec![
            ("SetDefine", set_define),
            ("SetTranslate", set_translate),
            ("SetStyle", set_style),
            ("GetDefine", get_define),
            ("GetTranslate", get_translate),
            ("GetStyle", get_style),
            ("Span", span),
            ("Resource", get_resource),
            ("Base64Resource", get_base64_resource),
        ];
        for (name, func) in functions {
            lua.globals().set(name, func).map_err(|e| RTError {
                code: ErrorCode::CannotCreateLuaGlobal,
                msg: format!("Cannot set global function name: {}!\nmessage: {}", name, e),
            })?;
        }
    }
    _ = lua
        .load(real_main_lua_content.clone())
        .exec()
        .map_err(|e| RTError {
            code: ErrorCode::CannotParseLuaFile,
            msg: format!("Cannot parse lua file!\nmessage: {}", e),
        })?;
    let save_directory = copywriting_struct
        .borrow()
        .define
        .get("config.save_directory")
        .ok_or(RTError {
            code: ErrorCode::CannotFindSaveDirectoryDefine,
            msg: format!(
                "Cannot find \"config.save_directory\" define! Please check your lua file."
            ),
        })?
        .as_str()
        .ok_or(RTError {
            code: ErrorCode::CannotFindSaveDirectoryDefine,
            msg: format!(
                "Cannot parse \"config.save_directory\" define key to string, Please check your lua file."
            ),
        })?.to_string();
    init_home_dir(&app_handle, save_directory.as_str()).map_err(|e| RTError {
        code: ErrorCode::CannotInitHomeDir,
        msg: format!("Cannot init home dir! message: {}", e),
    })?;
    drop(lua);
    // println!("{:?}", copywriting_struct);
    if let Ok(cell) = std::rc::Rc::try_unwrap(copywriting_struct) {
        Ok(cell.into_inner())
    } else {
        unreachable!()
    }
}
///
/// 遍历文件夹，找到所有 .rrs 的文件名。
///
#[tauri::command]
fn find_all_game_file_name() -> Result<Vec<String>, ErrorCode> {
    let execute_dir = get_executable_file_path().ok_or(ErrorCode::CannotGetExecuteDir)?;
    let mut engine_path = std::path::PathBuf::new();
    engine_path.push(execute_dir);
    engine_path.push("games");
    if !create_dir(engine_path.to_string_lossy().to_string().as_str()) {
        return Err(ErrorCode::CannotCreateGameDir);
    }
    let file_entry = std::fs::read_dir(engine_path).map_err(|_| ErrorCode::CannotReadGamesPath)?;
    let mut result: Vec<String> = Vec::new();
    for entry in file_entry {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if path.is_file() && ext == "rrs" {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let file_name = file_name.to_string_lossy().to_string();
            result.push(file_name);
        } else {
            continue;
        }
    }
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            init_copywriting,
            find_all_game_file_name
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
