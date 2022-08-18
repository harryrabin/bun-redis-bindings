use napi::{self};
use napi_derive::napi;
use redis::{Commands, ConnectionLike, RedisError, RedisResult};
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[napi]
pub struct RedisClient {
  client: redis::Client,
  connection: redis::Connection,
}

#[napi]
impl RedisClient {
  #[napi(constructor)]
  pub fn new(url: String) -> napi::Result<Self> {
    let client = match redis::Client::open(url) {
      Ok(client) => client,
      Err(e) => return Err(redis_to_napi_format(&e)),
    };

    let connection = match client.get_connection() {
      Ok(conn) => conn,
      Err(e) => return Err(redis_to_napi_format(&e)),
    };

    Ok(Self { client, connection })
  }

  #[napi]
  pub fn reconnect(&mut self) -> napi::Result<()> {
    match self.client.get_connection() {
      Ok(conn) => {
        self.connection = conn;
        napi::Result::Ok(())
      }
      Err(e) => Err(redis_to_napi_format(&e)),
    }
  }

  #[napi]
  pub fn cmd_get(&mut self, key: String) -> napi::Result<Option<String>> {
    redis_to_napi_optional(self.connection.get(key))
  }

  #[napi]
  pub fn cmd_set(&mut self, key: String, value: String) -> napi::Result<()> {
    redis_to_napi(self.connection.set(key, value))
  }

  #[napi]
  pub fn cmd_lpush(&mut self, key: String, value: Vec<String>) -> napi::Result<()> {
    redis_to_napi(self.connection.lpush(key, value))
  }

  #[napi]
  pub fn cmd_lpop(&mut self, key: String, count: u32) -> napi::Result<Option<Vec<String>>> {
    redis_to_napi_optional(self.connection.lpop(key, NonZeroUsize::new(count as usize)))
  }

  #[napi]
  pub fn cmd_hset(&mut self, key: String, field: String, value: String) -> napi::Result<()> {
    redis_to_napi(self.connection.hset(key, field, value))
  }

  #[napi]
  pub fn cmd_hget(&mut self, key: String, field: String) -> napi::Result<Option<String>> {
    redis_to_napi_optional(self.connection.hget(key, field))
  }

  #[napi]
  pub fn cmd_hgetall(&mut self, key: String) -> napi::Result<Option<HashMap<String, String>>> {
    redis_to_napi_optional(self.connection.hgetall(key))
  }

  #[napi]
  pub fn cmd_expire(&mut self, key: String, seconds: u32) -> napi::Result<()> {
    redis_to_napi(self.connection.expire(key, seconds as usize))
  }

  #[napi]
  pub fn cmd_del(&mut self, key: String) -> napi::Result<()> {
    redis_to_napi(self.connection.del(key))
  }

  #[napi]
  pub fn cmd_del_multiple(&mut self, keys: Vec<String>) -> napi::Result<()> {
    redis_to_napi(self.connection.del(keys))
  }

  #[napi]
  pub fn cmd_keys(&mut self, pattern: String) -> napi::Result<Vec<String>> {
    redis_to_napi(self.connection.keys(pattern))
  }

  #[napi]
  pub fn connection_open(&self) -> bool {
    self.connection.is_open()
  }
}

fn redis_to_napi<T>(result: RedisResult<T>) -> napi::Result<T> {
  match result {
    Ok(val) => Ok(val),
    Err(e) => Err(redis_to_napi_format(&e)),
  }
}

fn redis_to_napi_optional<T>(result: RedisResult<T>) -> napi::Result<Option<T>> {
  match result {
    Ok(val) => Ok(Some(val)),
    Err(e) => match e.kind() {
      redis::ErrorKind::TypeError => Ok(None),
      _ => Err(redis_to_napi_format(&e)),
    },
  }
}

fn redis_to_napi_format(err: &RedisError) -> napi::Error {
  napi::Error::new(
    napi::Status::Cancelled,
    format!(
      "Redis Error: {}\nDetails: {}\nCode: {}",
      err.category(),
      err.detail().unwrap_or("not specified"),
      err.code().unwrap_or("unknown"),
    ),
  )
}
