// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod vtop;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use futures_util::StreamExt;
use std::cmp::min;
use std::io::Write;
use tauri::Emitter;
use tauri::Window;
use tauri_plugin_http::reqwest;
use tokio::sync::Mutex;
use vtop::client::Iclient;
use vtop::parseattn;
use vtop::parsecoursepg;
use vtop::parsemarks;
use vtop::parsesched;
use vtop::parsett;
use vtop::wifi;

// (true or false, "msg")
// msg:
// (true)
// SS = suceesfully
// LA = loginactive
// (false)
// NE = networkerror {may be device offline}
// VE = vtop error
//

async fn login_vtop(client: &mut Iclient) -> (bool, String) {
    let login_pg = client.login_pageload().await;
    let mut ans_cap = "".to_string();
    if login_pg.0 {
        let url_safe_encoded_data = URL_SAFE.encode(&client.captcha);
        match reqwest::get(format!(
            "http://cap.synapticsurge.in/cap/{}", //cap.synapticsurge.in
            url_safe_encoded_data
        ))
        .await
        {
            Ok(res) => ans_cap = res.text().await.expect("failed"),
            Err(_e) => return (false, "NE".to_string()),
        }
    }
    let p = client.login(ans_cap).await;
    p
}
#[tauri::command]
async fn onstart_runcode(
    state: tauri::State<'_, Mutex<Iclient>>,
    username: String,
    password: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let m = client.loginactive;
    client.update_cred(username, password);
    let mut k = (m, "LA".to_string());
    if !m {
        k = login_vtop(&mut client).await;
    }
    Ok(k)
}
#[tauri::command]
async fn try_login(username: String, password: String) -> Result<(bool, String), tauri::Error> {
    let mut client = Iclient::new();
    let m = client.loginactive;
    client.update_cred(username, password);
    let mut k = (m, "".to_string());
    if !m {
        k = login_vtop(&mut client).await;
    }
    Ok(k)
}

#[tauri::command]
async fn update_credtials_to(
    state: tauri::State<'_, Mutex<Iclient>>,
    username: String,
    password: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    client.update_cred(username, password);
    Ok((true, "SS".to_string()))
}

#[tauri::command]
async fn timetable(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let check = login_vtop(&mut client).await;
        result = check;
    }
    let m = client.loginactive;
    if m {
        let res = client.get_timetable(semid).await;
        if res.0 {
            let k = parsett::parse_timetable(res.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = res;
        }
    }
    Ok(result)
}

#[tauri::command]
async fn timetable_semid(
    state: tauri::State<'_, Mutex<Iclient>>,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_timetable_page().await;
        if html.0 {
            let k = parsett::parse_semid_timetable(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn attendance_semid(
    state: tauri::State<'_, Mutex<Iclient>>,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_attendance_page().await;
        if html.0 {
            let k = parseattn::parse_semid_attendance(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn attendance(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_attendance(semid).await;
        if html.0 {
            let k = parseattn::parse_attendance(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn full_attendance(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
    course_id: String,
    course_type: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client
            .get_full_attendance(course_id, semid, course_type)
            .await;
        if html.0 {
            let k = parseattn::parse_full_attendance(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn wifi(i: i32, username: String, password: String) -> Result<(bool, String), tauri::Error> {
    let k = wifi::wifi_login_logout(i, username, password).await;
    Ok(k)
}

#[tauri::command]
async fn coursepage(
    state: tauri::State<'_, Mutex<Iclient>>,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_course_page().await;
        if html.0 {
            let k = parsecoursepg::parse_semid(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn coursepage_courses(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_course_courses(semid).await;
        if html.0 {
            let k = parsecoursepg::parse_courses(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn coursepage_classes(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
    classid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_course_classes(semid, classid).await;
        if html.0 {
            let k = parsecoursepg::parse_classes(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn coursepage_dlist(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
    classid: String,
    erp_id: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_course_dlist(semid, classid, erp_id).await;
        if html.0 {
            let k = parsecoursepg::parse_dlist(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn download_coursepage(
    state: tauri::State<'_, Mutex<Iclient>>,
    url: String,
    window: Window,
) -> Result<String, tauri::Error> {
    let mut client = state.lock().await;
    let username = client.username.clone();
    let csrf = client.csrf.clone();
    let cl = client.get_clone().await;
    drop(client);
    let mut filename = "file.zip".to_string();
    let u = format!(
        "https://vtop.vitap.ac.in/vtop/{}?authorizedID={}&_csrf={}",
        url, username, csrf
    );

    let k = cl.get(u).send().await;
    if let Ok(k) = k {
        if let Some(name) = k.headers().get(reqwest::header::CONTENT_DISPOSITION) {
            filename = name
                .to_str()
                .unwrap()
                .to_string()
                .split("filename=")
                .skip(1)
                .next()
                .unwrap_or("")
                .to_string()
                .replace(r#"""#, "");
        }

        let mut per = 0;
        if let Some(content_length) = k.content_length() {
            let mut downloaded: u64 = 0;
            let mut stream = k.bytes_stream();
            let file_path = format!("/storage/emulated/0/Download/{}", filename);
            let mut file = std::fs::File::create(file_path)?;
            while let Some(chunks) = stream.next().await {
                match chunks {
                    Ok(chunk) => {
                        file.write_all(&chunk).unwrap_or(());

                        let new = min(downloaded + (chunk.len() as u64), content_length);
                        downloaded = new;
                        let tper = downloaded * 100 / content_length;
                        if per != tper {
                            per = tper;
                            window.emit(&url, per).unwrap();
                        }
                    }
                    Err(_e) => (),
                }
            }
        } else {
            let mut stream = k.bytes_stream();
            let file_path = format!("/storage/emulated/0/Download/{}", filename);
            let file = std::fs::File::create(file_path);
            match file {
                Ok(mut file) => {
                    while let Some(chunks) = stream.next().await {
                        let chunk = chunks.unwrap();
                        file.write_all(&chunk).unwrap();
                        let tper = 50;
                        if per != tper {
                            per = tper;
                            window.emit(&url, per).unwrap();
                        }
                    }
                }
                Err(_e) => (),
            }
        }
    }
    Ok(filename)
}

#[tauri::command]
async fn marks_page(
    state: tauri::State<'_, Mutex<Iclient>>,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_marks_page().await;
        if html.0 {
            let k = parsemarks::parse_semid_marks(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn marks_list(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_marks_list(semid).await;
        if html.0 {
            let k = parsemarks::parse_marks(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}
#[tauri::command]
async fn exam_shedule_sems(
    state: tauri::State<'_, Mutex<Iclient>>,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_exam_shedule_sems().await;
        if html.0 {
            let k = parsesched::parse_semid_schedule(html.1);
            //println!("{}",k);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[tauri::command]
async fn exam_shedule(
    state: tauri::State<'_, Mutex<Iclient>>,
    semid: String,
) -> Result<(bool, String), tauri::Error> {
    let mut client = state.lock().await;
    let mut result = (false, "".to_string());

    let m = client.loginactive;
    if !m {
        let _check = login_vtop(&mut client).await;
        result = _check;
    }
    let m = client.loginactive;
    if m {
        let html = client.get_exam_shedule(semid).await;
        if html.0 {
            let k = parsesched::parse_schedule(html.1);
            if k == "[]" {
                result = (false, k);
            } else {
                result = (true, k)
            }
        } else {
            result = html;
        }
    }

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_nativeapi::init())
        .manage(Mutex::new(Iclient::new()))
        .invoke_handler(tauri::generate_handler![
            timetable_semid,
            timetable,
            attendance,
            full_attendance,
            attendance_semid,
            onstart_runcode,
            try_login,
            update_credtials_to,
            wifi,
            coursepage,
            coursepage_courses,
            coursepage_classes,
            coursepage_dlist,
            download_coursepage,
            marks_page,
            marks_list,
            exam_shedule,
            exam_shedule_sems,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
