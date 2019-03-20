use crate::ctapi::MAP;
use crate::{http, Status, CONFIG};
use failure::Error;

pub fn close(mut ctn: u16) -> Result<Status, Error> {
    if let Some(ctn_from_cfg) = CONFIG.read().ctn {
        debug!("Use ctn '{}' from configuration", ctn_from_cfg);
        ctn = ctn_from_cfg;
    }

    if !MAP.read().contains_key(&ctn) {
        error!("Card terminal has not been opened.");
        return Ok(Status::ERR_INVALID);
    }

    let pn = match MAP.read().get(&ctn) {
        None => return Err(format_err!("Failed to extract pn for given ctn!")),
        Some(pn) => *pn,
    };

    let path = format!("ct_close/{}/{}", ctn, pn);
    let response = http::request(&path, None)?;

    match response.parse::<i8>() {
        Ok(status) => {
            let status = Status::from_i8(status);
            if let Status::OK = status {
                // Remove CTN
                let _ = MAP.write().remove(&ctn);
                info!("Card terminal closed.");
            }

            Ok(status)
        }
        Err(why) => {
            debug!("{}", why);
            Err(format_err!("Unexpected server response found in body!"))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::close;
    use crate::ctapi::MAP;
    use crate::{Settings, Status, CONFIG};
    use rand;
    use std::collections::HashMap;
    use std::env;
    use test_server::{self, HttpResponse};

    #[test]
    fn returns_err_if_no_server() {
        env::set_var("K2_BASE_URL", "http://127.0.0.1:65432");
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();

        let _ = MAP.write().insert(ctn, pn);

        assert!(close(ctn).is_err());

        env::remove_var("K2_BASE_URL");
    }

    #[test]
    fn returns_err_invalid_if_already_closed() {
        let ctn = rand::random::<u16>();

        assert_eq!(Some(Status::ERR_INVALID), close(ctn).ok());
    }

    #[test]
    fn use_ctn_and_pn_in_request_path() {
        let server = test_server::new(0, |_| HttpResponse::BadRequest().into());
        env::set_var("K2_BASE_URL", server.url());
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();
        let _ = MAP.write().insert(ctn, pn);

        let _ = close(ctn);

        let path = server.requests.next().unwrap().path;
        assert_eq!(path, *format!("/ct_close/{}/{}", ctn, pn));

        env::remove_var("K2_BASE_URL");
    }

    #[test]
    fn use_ctn_and_pn_from_config() {
        let server = test_server::new(0, |_| HttpResponse::BadRequest().into());
        env::set_var("K2_BASE_URL", server.url());
        let ctn = rand::random::<u16>();
        env::set_var("K2_CTN", format!("{}", ctn));
        let pn = rand::random::<u16>();
        env::set_var("K2_PN", format!("{}", pn));
        init_config_clear_map();

        let _ = MAP.write().insert(ctn, pn);

        let unused_ctn = rand::random::<u16>();

        let _ = close(unused_ctn);

        let path = server.requests.next().unwrap().path;
        assert_eq!(path, *format!("/ct_close/{}/{}", ctn, pn));

        env::remove_var("K2_BASE_URL");
        env::remove_var("K2_CTN");
        env::remove_var("K2_PN");
    }

    #[test]
    fn returns_err_htsi_if_server_response_is_not_200() {
        let server = test_server::new(0, |_| HttpResponse::BadRequest().into());
        env::set_var("K2_BASE_URL", server.url());
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();
        let _ = MAP.write().insert(ctn, pn);

        assert!(close(ctn).is_err());
        assert_eq!(true, MAP.read().contains_key(&ctn));
    }

    #[test]
    fn returns_err_htsi_if_server_response_not_contains_status() {
        let server = test_server::new(0, |_| HttpResponse::Ok().body("hello world"));
        env::set_var("K2_BASE_URL", server.url());
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();
        let _ = MAP.write().insert(ctn, pn);

        assert!(close(ctn).is_err());
        assert_eq!(true, MAP.read().contains_key(&ctn));

        env::remove_var("K2_BASE_URL");
    }

    #[test]
    fn returns_response_status_from_server() {
        let server = test_server::new(0, |_| HttpResponse::Ok().body("-11"));
        env::set_var("K2_BASE_URL", server.url());
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();
        let _ = MAP.write().insert(ctn, pn);

        assert_eq!(Some(Status::ERR_MEMORY), close(ctn).ok());
        assert_eq!(true, MAP.read().contains_key(&ctn));

        env::remove_var("K2_BASE_URL");
    }

    #[test]
    fn return_ok_and_close_ctn_if_server_returns_ok() {
        let server = test_server::new(0, |_| HttpResponse::Ok().body("0"));
        env::set_var("K2_BASE_URL", server.url());
        init_config_clear_map();

        let ctn = rand::random::<u16>();
        let pn = rand::random::<u16>();
        let _ = MAP.write().insert(ctn, pn);

        assert_eq!(Some(Status::OK), close(ctn).ok());
        assert_eq!(false, MAP.read().contains_key(&ctn));

        env::remove_var("K2_BASE_URL");
    }

    fn init_config_clear_map() {
        let mut config_guard = CONFIG.write();
        *config_guard = Settings::init().unwrap();
        drop(config_guard);

        let mut map_guard = MAP.write();
        *map_guard = HashMap::new();
        drop(map_guard);
    }
}
