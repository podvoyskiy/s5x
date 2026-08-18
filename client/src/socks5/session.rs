use rustls::pki_types::ServerName;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tokio_rustls::TlsConnector;
use std::{str::FromStr, sync::Arc};

use crate::http::Http;
use crate::prelude::*;

pub enum ConnectTarget<'a> {
    FromConfig,
    Direct(&'a str)
}

pub struct Socks5Session {
    config: Config,
    pub server: Option<TcpStream>,
}

impl Socks5Session {
    pub fn new(config: Config, server: TcpStream) -> Self {
        Self { config, server: Some(server) }
    }

    pub async fn handshake(&mut self) -> Result<u8, AppError> {
        let mut methods = vec![consts::s5::auth::NO_AUTH];
        if self.config.auth.is_some() { methods.push(consts::s5::auth::AUTH); }

        let mut handshake = Vec::with_capacity(2 + methods.len());
        handshake.push(consts::s5::SOCKS_VERSION);
        handshake.push(u8::try_from(methods.len())?);
        handshake.extend_from_slice(&methods);

        utils::add_xor(self.config.xor, handshake.as_mut_slice());
        self.server.as_mut().unwrap().write_all(&handshake).await?;

        let mut buf = [0; 2];
        self.server.as_mut().unwrap().read_exact(&mut buf).await.map_err(|_| AppError::HandshakeFailed)?;
        trace!(?buf, "handshake");
        if buf[0] != consts::s5::SOCKS_VERSION || !methods.contains(&buf[1]) { 
            return Err(AppError::HandshakeFailed); 
        }
        Ok(buf[1])
    }

    pub async fn auth(&mut self) -> Result<(), AppError> {
        let (username, password) = self.config.auth.as_ref().unwrap();
        let mut auth = Vec::with_capacity(1 + 1 + username.len() + 1 + password.len());
        auth.push(consts::s5::auth::VERSION);
        auth.push(u8::try_from(username.len())?);
        auth.extend_from_slice(username.as_bytes());
        auth.push(u8::try_from(password.len())?);
        auth.extend_from_slice(password.as_bytes());

        utils::add_xor(self.config.xor, auth.as_mut_slice());
        self.server.as_mut().unwrap().write_all(&auth).await?;

        let mut buf = [0; 2];
        self.server.as_mut().unwrap().read_exact(&mut buf).await.map_err(|_| AppError::AuthFailed)?;
        trace!(?buf, "auth");

        if buf[0] != consts::s5::auth::VERSION || buf[1] != consts::s5::reply::SUCCESS { 
            return Err(AppError::AuthFailed); 
        }
        Ok(())
    }
    
    pub async fn connect(&mut self, target: ConnectTarget<'_>) -> Result<(), AppError> {
        let atyp = match target {
            ConnectTarget::FromConfig => {
                if self.config.mode != Mode::Socks5 { 
                    return Err(AppError::Socks5("target from config only for socks5 mode".into())); 
                }
                self.config.target.as_ref().ok_or(AppError::TargetUnreachable)?
            },
            ConnectTarget::Direct(target) => &Atyp::from_str(target)?,
        };
        let mut connect = vec![consts::s5::SOCKS_VERSION, consts::s5::connect::CMD, consts::s5::RSV];
        connect.extend_from_slice(&atyp.to_bytes());

        utils::add_xor(self.config.xor, connect.as_mut_slice());
        self.server.as_mut().unwrap().write_all(&connect).await?;

        let mut buf = [0; 10];
        self.server.as_mut().unwrap().read_exact(&mut buf).await.map_err(|_| AppError::ConnectFailed)?;
        trace!(?buf, "connect");

        if buf[0] != consts::s5::SOCKS_VERSION || buf[1] != consts::s5::reply::SUCCESS { 
            return Err(AppError::ConnectFailed); 
        }
        Ok(())
    }

    pub async fn send(&mut self) -> Result<(), AppError> {
        if self.config.use_tls { self.https().await } else { self.http().await }
    }

    async fn http(&mut self) -> Result<(), AppError> {
        let host = self.config.target.as_ref().unwrap().host_str();
        let mut stream = self.server.take().unwrap();

        let request = self.config.http.build_request(&host);
        stream.write_all(request.as_bytes()).await?;

        let response = Http::read_response(&mut stream).await?;
        Http::print_response(&response)
    }

    async fn https(&mut self) -> Result<(), AppError> {
        let host = self.config.target.as_ref().unwrap().host_str();
        let connector = Self::setup_tls_connector();
        let server_name = ServerName::try_from(host.clone())
            .map_err(|_| AppError::InvalidDomain)?;

        let mut tls_stream = connector.connect(server_name, self.server.take().unwrap()).await?;

        let request = self.config.http.build_request(&host);
        tls_stream.write_all(request.as_bytes()).await?;
        
        let response = Http::read_response(&mut tls_stream).await?;
        Http::print_response(&response)
    }

    fn setup_tls_connector() -> TlsConnector {
        //* loading root certificates
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        //* create tls client config
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        TlsConnector::from(Arc::new(config))
    }
}