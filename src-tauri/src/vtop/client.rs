use scraper::{Html, Selector};
use tauri_plugin_http::reqwest::{
    self,
    cookie::Jar,
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Client, Error,
};

// (true or false, "msg")
// msg:
// (true)
// SS = suceesfully
// (false)
// NE = networkerror {may be device offline}
// VE = vtop error
// RE = registraion number parsing error

pub struct Iclient {
    client: Client,
    online: bool,
    vtop_online: bool,
    current_page: String,
    pub csrf: String,
    pub username: String,
    password: String,
    pub loginactive: bool,
    pub captcha: String,
    captchans: String,
}

impl Iclient {
    pub fn new() -> Self {
        let client = Self::make_client();
        Iclient {
            client,
            online: true,
            vtop_online: true,
            current_page: "".to_string(),
            csrf: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            loginactive: false,
            captcha: "".to_string(),
            captchans: "".to_string(),
        }
    }
    pub fn update_cred(&mut self, username: String, password: String) {
        if self.username != username && self.password != password {
            self.username = username.to_uppercase();
            self.password = password;
            self.loginactive = false;
        }
    }
    fn make_client() -> Client {
        let mut headers = HeaderMap::new();
        let jar = Jar::default();
        let cookie_store = std::sync::Arc::new(jar);
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Linux; U; Linux x86_64; en-US) Gecko/20100101 Firefox/130.5",
            ),
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("en-US,en;q=0.5"),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
        headers.insert("Priority", HeaderValue::from_static("u=0, i"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .cookie_provider(cookie_store.clone())
            .build()
            .unwrap();
        return client;
    }

    async fn update_vtop_status(&mut self) -> Result<bool, Error> {
        match reqwest::get("https://vtop.vitap.ac.in/vtop").await {
            Ok(respons) => {
                if respons.status().is_success() {
                    self.vtop_online = true;
                    self.online = true;
                    Ok(true)
                } else {
                    self.vtop_online = false;
                    self.loginactive = false;
                    Ok(false)
                }
            }
            Err(_) => {
                self.online = false;
                Ok(false)
            }
        }
    }
    pub async fn get_clone(&mut self) -> Client {
        self.client.clone()
    }

    async fn initial_pageload(&mut self) -> Result<String, Error> {
        let body = self
            .client
            .get("https://vtop.vitap.ac.in/vtop/open/page")
            .send()
            .await?;
        let code = body.status().as_str().to_string();
        let data = body.text().await?;
        self.current_page = data;
        let _ = self.update_vtop_status().await;
        return Ok(code);
    }

    pub async fn login_pageload(&mut self) -> (bool, String) {
        let _ = self.update_vtop_status();
        if self.online == false {
            return (false, "NE".to_string());
        } else if self.vtop_online == false {
            return (false, "NE".to_string());
        }
        let _ = self.initial_pageload().await;
        self.get_csrf();
        let csrf = &self.csrf;
        let url = "https://vtop.vitap.ac.in/vtop/prelogin/setup";
        let body = format!("_csrf={csrf}&flag=VTOP");
        while self.vtop_online && self.online {
            let _ = self.update_vtop_status().await;
            match self.login_load(url, body.clone()).await {
                Ok(x) => {
                    self.current_page = x;
                    if let Some(p) = self.get_captcha_data() {
                        if p == "".to_string() {
                            continue;
                        }
                        self.captcha = p;
                        return (true, "SS".to_string());
                    }
                }
                Err(_e) => {
                    let _ = self.update_vtop_status().await;
                }
            }
        }
        return (false, "".to_string());
    }

    async fn get_regno(&mut self) -> (bool, String) {
        let document = Html::parse_document(&self.current_page);
        let selector = Selector::parse("input[type=hidden][name=authorizedIDX]").unwrap();

        let page = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value").map(|value| value.to_string()));
        match page {
            Some(x) => {
                return (true, x);
            }
            None => {
                return (false, "RE".to_string());
            }
        }
    }

    pub async fn login(&mut self, cap: String) -> (bool, String) {
        self.captchans = cap;
        let _ = self.update_vtop_status().await;
        if self.vtop_online {
            let login_re = self.login_request().await;
            if login_re.0 {
                self.current_page = login_re.1;
                let reg_p = self.get_regno().await;
                if reg_p.0 && reg_p.1 != self.username {
                    self.username = reg_p.1;
                }
            } else {
                return login_re;
            }
            if let Some(x) = self.get_login_page_error() {
                let x = x.trim().to_string();
                if let Some(p) = self.get_captcha_data() {
                    self.captcha = p;
                }
                return (false, x);
            } else {
                self.succeslogin();
                return (true, "SS".to_string());
            }
        } else {
            return (false, "VE".to_string());
        }
    }

    async fn login_request(&mut self) -> (bool, String) {
        let body = format!(
            "_csrf={}&username={}&password={}&captchaStr={}",
            self.csrf, self.username, self.password, self.captchans
        );
        let page = self
            .client
            .post("https://vtop.vitap.ac.in/vtop/login")
            .body(body)
            .send()
            .await;
        match page {
            Ok(k) => {
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
    async fn login_load(&mut self, url: &str, body: String) -> Result<String, reqwest::Error> {
        let text = self
            .client
            .post(url)
            .body(body)
            .send()
            .await?
            .text()
            .await?;
        Ok(text)
    }
    fn get_csrf(&mut self) {
        let document = Html::parse_document(&self.current_page);
        let selector = Selector::parse("input[type=hidden][name=_csrf]").unwrap();

        let page = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value").map(|value| value.to_string()));
        match page {
            Some(x) => {
                self.csrf = x;
            }
            None => {}
        }
    }
    fn get_captcha_data(&self) -> Option<String> {
        let document = Html::parse_document(&self.current_page);
        let selector = Selector::parse("img.form-control.img-fluid.bg-light.border-0").unwrap();
        document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("src").map(|src| src.to_string()))
    }

    pub fn get_login_page_error(&self) -> Option<String> {
        let ptext = r#"span.text-danger.text-center[role="alert"]"#;
        let document = Html::parse_document(&self.current_page);
        let selector = Selector::parse(&ptext).unwrap();
        if let Some(element) = document.select(&selector).next() {
            let error_message = element.text().collect::<Vec<_>>().join(" ");
            Some(error_message)
        } else {
            None
        }
    }
    fn succeslogin(&mut self) {
        self.get_csrf();
        self.loginactive = true;
    }

    pub async fn check(&mut self) -> bool {
        match self
            .client
            .get("https://vtop.vitap.ac.in/vtop/content?")
            .send()
            .await
        {
            Ok(respons) => {
                if respons.status().is_success() {
                    self.loginactive = true;
                    true
                } else {
                    self.loginactive = false;
                    false
                }
            }
            Err(_) => {
                self.online = false;
                false
            }
        }
    }

    pub async fn get_attendance_page(&self) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/academics/common/StudentAttendance";
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            self.username, self.csrf
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
    pub async fn get_attendance(&mut self, semid: String) -> (bool, String) {
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.csrf, semid, self.username
        );
        let url = "https://vtop.vitap.ac.in/vtop/processViewStudentAttendance";
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
    pub async fn get_full_attendance(
        &mut self,
        course_id: String,
        semid: String,
        course_type: String,
    ) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/processViewAttendanceDetail";
        let body = format!(
            "_csrf={}&semesterSubId={}&registerNumber={}&courseId={}&courseType={}&authorizedID={}",
            self.csrf, semid, self.username, course_id, course_type, self.username
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
    pub async fn get_timetable_page(&self) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/academics/common/StudentTimeTable";
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            self.username, self.csrf
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
    pub async fn get_timetable(&mut self, semid: String) -> (bool, String) {
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.csrf, semid, self.username
        );
        let url = "https://vtop.vitap.ac.in/vtop/processViewTimeTable";
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }

    pub async fn get_course_page(&self) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/academics/common/StudentCoursePage";
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            self.username, self.csrf
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }

    pub async fn get_course_courses(&self, semid: String) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/getCourseForCoursePage";
        let body = format!(
            "_csrf={}&paramReturnId=getCourseForCoursePage&semSubId={}&authorizedID={}",
            self.csrf, semid, self.username
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }

    pub async fn get_course_classes(&self, semid: String, classid: String) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/getSlotIdForCoursePage";
        let body = format!(
        "_csrf={}&classId={}&praType=source&paramReturnId=getSlotIdForCoursePage&semSubId={}&authorizedID={}",
        self.csrf,classid,semid,self.username
    );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }

    pub async fn get_course_dlist(
        &self,
        semid: String,
        classid: String,
        erp_id: String,
    ) -> (bool, String) {
        let url = "https://vtop.vitap.ac.in/vtop/processViewStudentCourseDetail";
        let body = format!(
            "_csrf={}&semSubId={}&erpId={}&classId={}&authorizedID={}",
            self.csrf, semid, erp_id, classid, self.username
        );
        let res = self.client.post(url).body(body).send().await;
        match res {
            Ok(k) => {
                if !k.status().is_success() {
                    return (false, "VE".to_string());
                }
                let t = k.text().await;
                match t {
                    Ok(k) => {
                        return (true, k);
                    }
                    Err(_) => {
                        return (false, "VE".to_string());
                    }
                };
            }
            Err(_) => {
                return (false, "NE".to_string());
            }
        }
    }
}
