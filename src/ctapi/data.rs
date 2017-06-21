extern crate serde_json;

use self::super::{ERR_HTSI, ERR_INVALID, MAP, OK};
use self::super::super::http;

use base64::{encode, decode};
use std::slice;

#[derive(Serialize)]
struct RequestData {
    dad: u8,
    sad: u8,
    lenc: u16,
    command: String,
    lenr: u16,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct ResponseData {
    dad: u8,
    sad: u8,
    lenr: u16,
    response: String,
    responseCode: i8,
}

pub fn data(
    ctn: u16,
    dad: *mut u8,
    sad: *mut u8,
    lenc: u16,
    command: *const u8,
    lenr: *mut u16,
    response: *mut u8,
) -> i8 {

    let safe_dad: &mut u8 = unsafe { &mut *dad };
    debug!("dad: {}", safe_dad);

    let safe_sad: &mut u8 = unsafe { &mut *sad };
    debug!("sad: {}", safe_sad);
    debug!("lenc: {}", lenc);

    let safe_command = unsafe { slice::from_raw_parts(command, lenc as usize) };
    debug!("command: {:?}", safe_command);

    let safe_lenr: &mut u16 = unsafe { &mut *lenr };
    debug!("lenr: {}", safe_lenr);

    let safe_response = unsafe { slice::from_raw_parts_mut(response, *safe_lenr as usize) };
    debug!("response with {} slices formed", safe_response.len());

    if !MAP.lock().unwrap().contains_key(&ctn) {
        error!(
            "Card terminal has not been opened. Returning {}",
            ERR_INVALID
        );
        return ERR_INVALID;
    }

    let request_data = RequestData {
        dad: *safe_dad,
        sad: *safe_sad,
        lenc,
        command: encode(safe_command),
        lenr: *safe_lenr,
    };

    let path = get_request_path(ctn);
    let request_body = serde_json::to_string(&request_data).unwrap();
    let response = http::request().post(&path, Some(request_body)).response();

    if response.status() != 200 {
        error!("Request failed! Returning {}", ERR_HTSI);
        return ERR_HTSI;

    }

    let data: ResponseData = match serde_json::from_str(&response.body()) {
        Ok(response) => response,
        Err(error) => {
            error!("Failed to parse response data. {}", error);
            error!("Returning {}", ERR_HTSI);
            return ERR_HTSI;
        }
    };

    if data.responseCode == OK {
        *safe_dad = data.dad;
        *safe_sad = data.sad;
        *safe_lenr = data.lenr;

        let decoded = decode(&data.response).unwrap();
        debug!("Decoded response field {:?}", decoded);

        for (place, element) in safe_response.iter_mut().zip(decoded.iter()) {
            *place = *element;
        }
    }
    debug!("Returning {}", data.responseCode);
    return data.responseCode;
}

fn get_request_path(ctn: u16) -> String {
    let pn = MAP.lock().unwrap();
    let pn = pn.get(&ctn).unwrap();

    format!("ct_data/{}/{}", ctn, pn)
}
