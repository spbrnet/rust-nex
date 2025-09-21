use macros::RmcSerialize;

#[derive(RmcSerialize)]
#[derive(Clone)]
pub struct Account{
    pub pid: u32,
    pub username: String,
    pub kerbros_password: [u8; 16],
}

impl Account{
    pub fn new(pid: u32, username: &str, passwd: &str) -> Self{
        let passwd_data = passwd.as_bytes();

        let mut passwd = [0u8; 16];

        for (idx, byte) in passwd_data.iter().enumerate(){
            passwd[idx] = *byte;
        }

        Self{
            kerbros_password: passwd,
            username: username.into(),
            pid
        }
    }

    pub fn new_raw_password(pid: u32, username: &str, passwd: [u8; 16]) -> Self{
        Self{
            kerbros_password: passwd,
            username: username.into(),
            pid
        }
    }

    pub fn get_login_data(&self) -> (u32, [u8; 16]){
        (self.pid, self.kerbros_password)
    }
}