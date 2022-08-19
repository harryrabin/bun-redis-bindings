use napi::{
  self,
  bindgen_prelude::{Either, Either3, Null},
};
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
      Err(e) => return Err(redis_err_to_napi_err(&e)),
    };

    let connection = match client.get_connection() {
      Ok(conn) => conn,
      Err(e) => return Err(redis_err_to_napi_err(&e)),
    };

    Ok(Self { client, connection })
  }

  #[napi]
  pub fn reconnect(&mut self) -> napi::Result<()> {
    match self.client.get_connection() {
      Ok(conn) => {
        self.connection = conn;
        Ok(())
      }
      Err(e) => Err(redis_err_to_napi_err(&e)),
    }
  }

  #[napi]
  pub fn connection_open(&self) -> bool {
    self.connection.is_open()
  }

  // HIGH-LEVEL BINDINGS

  #[napi]
  pub fn get(
    &mut self,
    key: String,
  ) -> napi::Result<Either3<String, HashMap<String, String>, Null>> {
    let field_type: String = match redis::cmd("TYPE").arg(&key).query(&mut self.connection) {
      Ok(val) => val,
      Err(e) => return Err(redis_err_to_napi_err(&e)),
    };

    match field_type.as_str() {
      "none" => Ok(Either3::C(Null)),
      "string" => match self.connection.get(&key) {
        Ok(val) => Ok(Either3::A(val)),
        Err(e) => Err(redis_err_to_napi_err(&e)),
      },
      "hash" => match self.connection.hgetall(&key) {
        Ok(val) => Ok(Either3::B(val)),
        Err(e) => Err(redis_err_to_napi_err(&e)),
      },
      _ => Err(napi::Error::new(
        napi::Status::Unknown,
        "field type unknown".to_string(),
      )),
    }
  }

  // COMMAND BINDINGS

  // Unsafe

  fn execute<T: redis::FromRedisValue>(&mut self, args: &Vec<String>) -> RedisResult<T> {
    redis::Cmd::new().arg(args).query(&mut self.connection)
  }

  #[napi]
  pub fn expect_string(&mut self, args: Vec<String>) -> napi::Result<String> {
    redis_to_napi(self.execute(&args))
  }

  #[napi]
  pub fn expect_array(&mut self, args: Vec<String>) -> napi::Result<Vec<String>> {
    redis_to_napi(self.execute(&args))
  }

  #[napi]
  pub fn expect_integer(&mut self, args: Vec<String>) -> napi::Result<u32> {
    redis_to_napi(self.execute(&args))
  }

  #[napi]
  pub fn expect_nil(&mut self, args: Vec<String>) -> napi::Result<()> {
    redis_to_napi(self.execute(&args))
  }

  // Getters/setters

  #[napi(js_name = "cmdGET")]
  pub fn cmd_get(&mut self, key: String) -> napi::Result<Option<String>> {
    redis_to_napi_optional(self.connection.get(key))
  }

  #[napi(js_name = "cmdSET")]
  pub fn cmd_set(&mut self, key: String, value: String) -> napi::Result<()> {
    redis_to_napi(self.connection.set(key, value))
  }

  #[napi(js_name = "cmdLPUSH")]
  pub fn cmd_lpush(&mut self, key: String, value: Vec<String>) -> napi::Result<()> {
    redis_to_napi(self.connection.lpush(key, value))
  }

  #[napi(js_name = "cmdLPOP")]
  pub fn cmd_lpop(&mut self, key: String, count: u32) -> napi::Result<Option<Vec<String>>> {
    redis_to_napi_optional(self.connection.lpop(key, NonZeroUsize::new(count as usize)))
  }

  #[napi(js_name = "cmdHSET")]
  pub fn cmd_hset(&mut self, key: String, field: String, value: String) -> napi::Result<()> {
    redis_to_napi(self.connection.hset(key, field, value))
  }

  #[napi(js_name = "cmdHGET")]
  pub fn cmd_hget(&mut self, key: String, field: String) -> napi::Result<Option<String>> {
    redis_to_napi_optional(self.connection.hget(key, field))
  }

  #[napi(js_name = "cmdHGETALL")]
  pub fn cmd_hgetall(&mut self, key: String) -> napi::Result<Option<HashMap<String, String>>> {
    redis_to_napi_optional(self.connection.hgetall(key))
  }

  // Utilities

  #[napi(js_name = "cmdEXPIRE")]
  pub fn cmd_expire(&mut self, key: String, seconds: u32) -> napi::Result<u32> {
    redis_to_napi(self.connection.expire(key, seconds as usize))
  }

  #[napi(js_name = "cmdDEL")]
  pub fn cmd_del(&mut self, key: Either<String, Vec<String>>) -> napi::Result<u32> {
    redis_to_napi(match key {
      Either::A(val) => self.connection.del(val),
      Either::B(val) => self.connection.del(val),
    })
  }

  #[napi(js_name = "cmdKEYS")]
  pub fn cmd_keys(&mut self, pattern: String) -> napi::Result<Vec<String>> {
    redis_to_napi(self.connection.keys(pattern))
  }

  #[napi(js_name = "cmdTYPE")]
  pub fn cmd_type(&mut self, key: String) -> napi::Result<String> {
    redis_to_napi(redis::cmd("TYPE").arg(key).query(&mut self.connection))
  }
}

fn redis_to_napi<T: redis::FromRedisValue>(result: RedisResult<T>) -> napi::Result<T> {
  match result {
    Ok(val) => Ok(val),
    Err(e) => Err(redis_err_to_napi_err(&e)),
  }
}

fn redis_to_napi_optional<T: redis::FromRedisValue>(
  result: RedisResult<T>,
) -> napi::Result<Option<T>> {
  match result {
    Ok(val) => Ok(Some(val)),
    Err(e) => match e.kind() {
      redis::ErrorKind::TypeError | redis::ErrorKind::ExtensionError => Ok(None),
      _ => Err(redis_err_to_napi_err(&e)),
    },
  }
}

fn redis_err_to_napi_err(err: &RedisError) -> napi::Error {
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
