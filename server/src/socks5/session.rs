use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use crate::{prelude::*, socks5::{parse, config::Socks5Config}};

#[derive(Debug, PartialEq)]
enum Socks5State {
    Handshake,
    Auth,
    Connect,
    Tunneling
}

#[derive(Debug)]
pub struct Socks5Session {
    config: Socks5Config,
    state: Socks5State,
    client: Option<TcpStream>,
    target: Option<TcpStream>,
}

impl Socks5Session {
    pub fn new(config: Socks5Config, client: TcpStream) -> Self {
        Self { config, state: Socks5State::Handshake, client: Some(client), target: None }
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        let mut buf = [0; 4096];

        loop {
            match self.client.as_mut().unwrap().read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if self.state != Socks5State::Tunneling { utils::add_xor(self.config.xor, &mut buf[..n]); }
                    match self.state {
                        Socks5State::Handshake => self.handshake(&buf[..n]).await?,
                        Socks5State::Auth => self.auth(&buf[..n]).await?,
                        Socks5State::Connect => self.connect(&buf[..n]).await?,
                        Socks5State::Tunneling => {
                            self.tunneling(&buf[..n]).await?;
                            break;
                        },
                    }
                },
                Err(e) => return Err(AppError::Socks5(format!("read error: {e}"))),
            }
        }

        Ok(())
    }

    async fn handshake(&mut self, buf: &[u8]) -> Result<(), AppError> {
        trace!(buf, "handshake");
        if buf.len() < 3 || buf[0] != consts::SOCKS_VERSION { return Err(AppError::HandshakeFailed); }
        let methods = buf.get(2..2 + buf[1] as usize).ok_or(AppError::HandshakeFailed)?;

        if self.config.auth.is_some() && methods.contains(&consts::auth::AUTH) {
            self.state = Socks5State::Auth;
            self.client.as_mut().unwrap().write_all(&[consts::SOCKS_VERSION, consts::auth::AUTH]).await?;
            Ok(())
        } else if self.config.auth.is_none() && methods.contains(&consts::auth::NO_AUTH) {
            self.state = Socks5State::Connect;
            self.client.as_mut().unwrap().write_all(&[consts::SOCKS_VERSION, consts::auth::NO_AUTH]).await?;
            Ok(())
        } else {
            self.client.as_mut().unwrap().write_all(&[consts::SOCKS_VERSION, consts::reply::NO_ACCEPTABLE_METHOD]).await?;
            Err(AppError::HandshakeFailed)
        }
    }

    async fn auth(&mut self, buf: &[u8]) -> Result<(), AppError> {
        trace!(buf, "auth");
        if buf.first() != Some(&consts::auth::VERSION) { return Err(AppError::AuthFailed); }

        let (user, pass) = parse::bytes_to_credentials(buf)?;
        let (user_config, pass_config) = self.config.auth.as_ref().unwrap();

        if &user != user_config || &pass != pass_config {
            warn!(username = ?user, password = ?pass, "auth failed. invalid credentials");
            self.client.as_mut().unwrap().write_all(&[consts::auth::VERSION, consts::reply::FAILURE]).await?;
            return Err(AppError::AuthFailed);
        }
        
        self.state = Socks5State::Connect;
        self.client.as_mut().unwrap().write_all(&[consts::auth::VERSION, consts::reply::SUCCESS]).await?;
        Ok(())
    }

    async fn connect(&mut self, buf: &[u8]) -> Result<(), AppError> {
        trace!(buf, "connect");
        if buf.len() < 4 || buf[0] != consts::SOCKS_VERSION || buf[1] != consts::connect::CMD { return Err(AppError::ConnectFailed); }

        let atyp = Atyp::from_bytes(buf.get(3..).ok_or(AppError::ConnectFailed)?.to_vec())?;
        let target_addr = atyp.to_socket_addr();

        let mut response = Vec::with_capacity(10);

        if let Ok(target_addr) = target_addr {
            let stream = TcpStream::connect(target_addr).await?;
            self.target = Some(stream);

            info!(target = ?target_addr, "connected to");
            
            response.extend_from_slice(&[consts::SOCKS_VERSION, consts::reply::SUCCESS, consts::RSV]);
            response.push(if target_addr.is_ipv4() { consts::connect::ATYP_IPV4 } else { consts::connect::ATYP_IPV6 });
            response.extend(parse::addr_to_bytes(self.target.as_ref().unwrap())?);

            self.state = Socks5State::Tunneling;

            self.client.as_mut().unwrap().write_all(&response).await?;
            Ok(())
        } else {
            warn!("failed to connect to any target address");

            response.extend_from_slice(&[consts::SOCKS_VERSION, consts::reply::FAILURE, consts::RSV, consts::connect::ATYP_IPV4]);
            response.extend_from_slice(consts::reply::BND_ADDR);
            response.extend_from_slice(consts::reply::BND_PORT);

            self.client.as_mut().unwrap().write_all(&response).await?;
            Err(AppError::TargetUnreachable)
        }
    }

    async fn tunneling(&mut self, buf: &[u8]) -> Result<(), AppError> {
        self.target.as_mut().unwrap().write_all(buf).await?;
                            
        let (mut client_r, mut client_w) = self.client.take().unwrap().into_split();
        let (mut target_r, mut target_w) = self.target.take().unwrap().into_split();

        let client_to_target = tokio::spawn(async move {
            if let Err(e) = tokio::io::copy(&mut client_r, &mut target_w).await {
                debug!("client->target copy error: {e}");
            }
        });

        let target_to_client = tokio::spawn(async move {
            if let Err(e) = tokio::io::copy(&mut target_r, &mut client_w).await {
                debug!("target->client copy error: {e}");
            }
        });

        let _ = tokio::join!(client_to_target, target_to_client);
        Ok(())
    }
}