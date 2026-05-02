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

#[derive(serde::Deserialize, serde::Serialize)]
enum ErrorCode {
    CannotGetExecuteDir = -1,
    CannotReadGamesPath = -2,
    CannotCreateGameDir = -3,
    CannotDecryptData = -4,
    CannotParseLuaFile = -5,
    CannotInitHomeDir = -10000,
    AesKeyHasWrong = -2147483647,
    NotImplements = -2147483648,
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct RTError {
    msg: String,
    code: ErrorCode,
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct CopywritingStruct {
    // 全局定义
    define: serde_json::Map<String, serde_json::Value>,
    // 部分二进制资源。
    resource: BTreeMap<String, String>,
    // 当前语言文件里面的总数
    locale: BTreeMap<String, String>,
    // 当前翻译
    translate: BTreeMap<String, BTreeMap<String, String>>,
    // 当前样式表
    style: serde_json::Map<String, serde_json::Value>,
    // 最终的文案代码！
    copywriting: serde_json::Map<String, serde_json::Value>,
}
pub static COPY_WRITING: std::sync::OnceLock<CopywritingStruct> = std::sync::OnceLock::new();
// 遍历当前 LuaTable，将当前 LuaTable 转成 serde_json 格式。
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
fn lua_value_to_json(key: String, value: mlua::Value) -> Result<serde_json::Value, mlua::Error> {
    match value {
        mlua::Value::String(s) => {
            let str = s
                .to_str()
                .map_err(|_| {
                    mlua::Error::runtime(format!(
                        "Cannot convert Define code to string! key: {}",
                        key
                    ))
                })?
                .to_string();
            Ok(serde_json::Value::String(str))
        }
        mlua::Value::Number(n) => {
            let num = serde_json::Number::from_f64(n).ok_or(mlua::Error::runtime(format!(
                "Cannot convert Define code to f64! key: {}",
                key
            )))?;
            Ok(serde_json::Value::Number(num))
        }
        mlua::Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        mlua::Value::Integer(i) => Ok(i.into()),
        mlua::Value::Table(t) => Ok(recursion_lua_table_to_json(key, &t)?),
        _ => {
            return Err(mlua::Error::runtime(format!(
                "Cannot convert Define a valid value! key: {}",
                key
            )));
        }
    }
}
#[tauri::command]
fn init_copywriting(
    app_handle: tauri::AppHandle,
    file_name: String,
) -> Result<CopywritingStruct, RTError> {
    use base64::Engine;
    let execute_dir = get_executable_file_path().ok_or(RTError {
        code: ErrorCode::CannotInitHomeDir,
        msg: "Cannot get executable file path!".to_string(),
    })?;
    let mut engine_path = std::path::PathBuf::new();
    engine_path.push(execute_dir);
    engine_path.push("games");
    engine_path.push(file_name);
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(AES_KEY)
        .map_err(|_| RTError {
            code: ErrorCode::AesKeyHasWrong,
            msg: "AES Key is fault! please enter a real AES Key!".to_string(),
        })?;
    if key_bytes.len() != 32 {
        return Err(RTError {
            code: ErrorCode::AesKeyHasWrong,
            msg: "AES Key is fault! please enter a real AES Key!".to_string(),
        });
    }
    let key_slice: [u8; 32] = key_bytes.try_into().unwrap();
    let engine_path = engine_path.to_string_lossy().to_string();
    let (file_map, main_lua_file_name) =
        decrypt_to_memory(engine_path, key_slice).map_err(|_| RTError {
            code: ErrorCode::CannotDecryptData,
            msg: "Cannot decrypt your data, might be the aes key not equal the decrypt data!"
                .to_string(),
        })?;
    let Some(main_lua_file_name) = main_lua_file_name else {
        return Err(RTError {
            code: ErrorCode::CannotDecryptData,
            msg: "Cannot decrypt your data, might be the aes key not equal the decrypt data!"
                .to_string(),
        });
    };
    let main_lua_content =
        String::from_utf8(file_map[&main_lua_file_name].clone()).map_err(|_| RTError {
            code: ErrorCode::CannotParseLuaFile,
            msg: "Your lua file have a wrong encoding! please try again!".to_string(),
        })?;
    let real_main_lua_content = parse_embed(
        main_lua_file_name.as_str(),
        main_lua_content.as_str(),
        &file_map.clone(),
    );
    // println!("{}", real_main_lua_content);
    let copywriting_struct: std::rc::Rc<std::cell::RefCell<CopywritingStruct>> =
        std::rc::Rc::new(std::cell::RefCell::new(CopywritingStruct {
            define: serde_json::Map::new(),
            resource: BTreeMap::new(),
            locale: BTreeMap::new(),
            translate: BTreeMap::new(),
            style: serde_json::Map::new(),
            copywriting: serde_json::Map::new(),
        }));
    {
        use mlua::prelude::*;
        let lua = Lua::new();
        let set_define_borrow = copywriting_struct.clone();
        let set_define = lua.create_function(move |_: &Lua, (key, value): (String, LuaValue)| {
            set_define_borrow
                .borrow_mut()
                .define
                .insert(key.clone(), lua_value_to_json(key.clone(), value)?);
            Ok(())
        });
        // let
        let set_locale_borrow = copywriting_struct.clone();
        let set_locale: Result<LuaFunction, LuaError> =
            lua.create_function(move |_: &Lua, (key, value): (String, String)| {
                set_locale_borrow
                    .borrow_mut()
                    .locale
                    .insert(key.clone(), value.clone());
                Ok(())
            });
        let set_translate_borrow = copywriting_struct.clone();
        let set_translate = lua.create_function(
            move |_: &Lua, (translate_key, key, value): (String, String, String)| {
                let translate_kvargs = &mut set_translate_borrow.borrow_mut().translate;
                if !translate_kvargs.contains_key(translate_key.as_str()) {
                    translate_kvargs.insert(translate_key.clone(), BTreeMap::new());
                }
                let translate_kvargs = translate_kvargs.get_mut(translate_key.as_str()).unwrap();
                translate_kvargs.insert(key.clone(), value.clone());
                Ok(())
            },
        );
        let set_style_borrow = copywriting_struct.clone();
        let set_style =
            lua.create_function(
                move |_: &Lua, (key, value): (String, LuaValue)| match value {
                    mlua::Value::String(s) => {
                        set_style_borrow.borrow_mut().style.insert(
                            key,
                            serde_json::Value::String(s.to_string_lossy().to_string()),
                        );
                        Ok(())
                    }
                    mlua::Value::Table(t) => {
                        let mut t_map = serde_json::Map::new();
                        for pair in t.pairs::<mlua::Value, mlua::Value>() {
                            let (k, v) = pair?;
                            if let mlua::Value::String(s) = k {
                                if let mlua::Value::String(s2) = v {
                                    t_map.insert(
                                        s.to_string_lossy().to_string(),
                                        serde_json::Value::String(s2.to_string_lossy().to_string()),
                                    );
                                } else {
                                    return Err(mlua::Error::RuntimeError(format!(
                                        "Connot convert style value {:?} in this key {}",
                                        v.to_string(),
                                        key
                                    )));
                                }
                            } else {
                                return Err(mlua::Error::RuntimeError(format!(
                                    "Connot convert style key {:?} in this key {}",
                                    k.to_string(),
                                    key
                                )));
                            }
                        }
                        set_style_borrow
                            .borrow_mut()
                            .style
                            .insert(key, serde_json::Value::Object(t_map));
                        Ok(())
                    }
                    _ => Err(mlua::Error::RuntimeError(format!(
                        "Cannot convert Define a valid value! key: {}",
                        key
                    ))),
                },
            );
        let get_define =
            lua.create_function(|_: &Lua, key: String| Ok(format!("<g-define>{}</g-define>", key)));
        let get_translate = lua.create_function(|_: &Lua, key: String| {
            Ok(format!("<g-translate>{}</g-translate>", key))
        });
        let get_style =
            lua.create_function(|_: &Lua, key: String| Ok(format!("<g-style>{}</g-style>", key)));
        let get_image_borrow = copywriting_struct.clone();
        let get_image = lua.create_function(move |_: &Lua, (key, img_type): (String, String)| {
            get_image_borrow
                .borrow_mut()
                .resource
                .insert(key.clone(), format!("data:{};base64,{}", img_type, key));
            Ok(key.clone())
        });
    }
    Err(RTError {
        code: ErrorCode::NotImplements,
        msg: "Code not implementation! please wait it first version upload!".to_string(),
    })
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
