pub struct LuaAnnotation(pub &'static str);
inventory::collect!(LuaAnnotation);

pub struct LuaAnnotationGetter(pub fn() -> String);
inventory::collect!(LuaAnnotationGetter);

pub struct LuaSetupGlobal(pub fn(&mut mlua::Table) -> mlua::Result<()>);
inventory::collect!(LuaSetupGlobal);

pub trait LuaType {
  const NAME: &'static str;
}

#[macro_export]
macro_rules! lua_type {
  ($type:ty, $lua_type:expr) => {
    impl $crate::ui::common::lua::lua_macros::LuaType for $type {
      const NAME: &'static str = $lua_type;
    }
    impl $crate::ui::common::lua::lua_macros::LuaType for std::option::Option<$type> {
      const NAME: &'static str = constcat::concat!($lua_type, "?");
    }
    impl $crate::ui::common::lua::lua_macros::LuaType for std::vec::Vec<$type> {
      const NAME: &'static str = constcat::concat!($lua_type, "[]");
    }
  };
}

lua_type!(bool, "boolean");
lua_type!(f32, "number");
lua_type!(f64, "number");
lua_type!(u8, "integer");
lua_type!(u16, "integer");
lua_type!(u32, "integer");
lua_type!(u64, "integer");
lua_type!(usize, "integer");
lua_type!(i8, "integer");
lua_type!(i16, "integer");
lua_type!(i32, "integer");
lua_type!(i64, "integer");
lua_type!(isize, "integer");
lua_type!(String, "string");
lua_type!(&str, "string");
lua_type!((), "nil");

#[macro_export]
macro_rules! to_lua_type {
  ($t:ty) => {
    <$t as $crate::ui::common::lua::lua_macros::LuaType>::NAME
  };
}

#[macro_export]
macro_rules! lua_enum {
  ($name:path, $ty:ty, $lua_name:literal) => {
    impl mlua::IntoLua for $name {
      fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        (self as $ty).into_lua(lua)
      }
    }

    impl mlua::FromLua for $name {
      fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let value = <$ty>::from_lua(value, lua)?;

        <$name>::from_repr(value).ok_or(mlua::Error::FromLuaConversionError {
          from: stringify!($ty),
          to: stringify!($name).to_string(),
          message: Some(format!("Invalid enum variant: {}", value)),
        })
      }
    }

    $crate::lua_type!($name, $lua_name);

    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotationGetter(|| {
        let values = <$name as strum::IntoEnumIterator>::iter()
          .map(|variant| format!("  {} = {:#04X},", Into::<&str>::into(variant), variant as $ty))
          .collect::<Vec<_>>()
          .join("\n");

        format!(
          indoc::indoc!("
            ---@enum {name}
            {name} = {{
            {values}
            }}

          "),
          name = $lua_name,
          values = values,
        )
      })
    }

    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaSetupGlobal(|globals| {
        let table = <$name as strum::IntoEnumIterator>::iter()
          .map(|variant| (Into::<&str>::into(variant), variant as $ty))
          .collect::<std::collections::HashMap<_, _>>();

        globals.set($lua_name, table)
      })
    }
  };
}

#[macro_export]
macro_rules! lua_func {
  ($name:ident, ($($arg:ident : $ty:ty),*) -> $ret:ty) => {
    pub struct $name {
      inner: mlua::Function,
      _marker: std::marker::PhantomData<fn($($ty),*) -> $ret>,
    }

    impl $name {
      #[allow(unused, unused_parens)]
      pub fn call(&self, args: ($($ty),*)) -> mlua::Result<$ret> {
        self.inner.call(args)
      }
    }

    impl mlua::IntoLua for $name {
      fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        self.inner.into_lua(lua)
      }
    }

    impl mlua::FromLua for $name {
      fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Self {
          inner: mlua::Function::from_lua(value, lua)?,
          _marker: std::marker::PhantomData,
        })
      }
    }

    $crate::lua_type!($name, stringify!($name));

    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        "---@alias ",
        stringify!($name),
        " fun(",
        $(
          constcat::concat![stringify!($arg), " : ", $crate::to_lua_type!($ty)],
        )*
        "): ",
        $crate::to_lua_type!($ret),
        "\n\n",
      ])
    }
  };
}

#[macro_export]
macro_rules! lua_struct {
  (
    $(#[doc = $struct_doc:literal])?
    struct $name:ident {
      $(
        $(#[doc = $field_doc:literal])?
        $field:ident : $ty:ty
      ),* $(,)?
    }
  ) => {
    #[derive(Clone)]
    pub struct $name {
      $(pub $field: $ty),*
    }

    impl mlua::UserData for $name {
      fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        $(
          fields.add_field_method_get(stringify!($field), |_, this| Ok(this.$field.clone()));
        )*
      }
    }

    impl mlua::UserData for &$name {
      fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        $(
          fields.add_field_method_get(stringify!($field), |_, this| Ok(this.$field.clone()));
        )*
      }
    }

    $crate::lua_type!($name, stringify!($name));
    $crate::lua_type!(&$name, stringify!($name));

    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $struct_doc,
          "\n",
        )?
        "---@class ",
        stringify!($name),
        "\n",
        $(
          constcat::concat![
            "---@field ",
            stringify!($field),
            " ",
            $crate::to_lua_type!($ty),
            $(
              constcat::concat![
                " -- ",
                $field_doc,
              ],
            )?
            "\n",
          ],
        )*
        "\n\n",
      ])
    }
  };
}

#[macro_export]
macro_rules! lua_annotations {
  ({
    $(#[doc = $doc:literal])?
    $table:ident = { $($inner:tt)* }
  }) => {
    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $doc,
          "\n",
        )?
        "---@class ",
        stringify!($table),
        "\n",
        stringify!($table),
        " = {{}}\n\n",
      ])
    }

    // recurse into inner items
    lua_annotations!(@table lua gw_api $table [$table] { $($inner)* });
  };

  (
    @table $lua:ident $gw:ident $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $table:ident = { $($inner:tt)* },
      $($rest:tt)*
    }
  ) => {
    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $doc,
          "\n",
        )?
        "---@class ",
        stringify!($table),
        "\n",
        constcat::concat![
          $(stringify!($path), "."),+,
          stringify!($table),
        ],
        " = {{}}\n\n"
      ])
    }

    lua_annotations!(@table $lua $gw $table [$($path).+ . $table] { $($inner)* });

    lua_annotations!(@table $lua $gw $parent [$($path).+] { $($rest)* });
  };

  (
    @table $lua:ident $gw:ident $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $func:ident = async || -> Result<$ret:ty> $body:block,
      $($rest:tt)*
    }
  ) => {
    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $doc,
          "\n",
        )?
        "---@return ",
        $crate::to_lua_type!($ret),
        "\n",
        "function ",
        constcat::concat![
          $(stringify!($path), "."),+,
          stringify!($func),
        ],
        "() end\n\n",
      ])
    }

    lua_annotations!(@table $lua $gw $parent [$($path).+] { $($rest)* });
  };

  (
    @table $lua:ident $gw:ident $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $func:ident = async |$(#[doc = $param_doc:literal])? $param:ident : mlua::Variadic<$ty:ty>| -> Result<$ret:ty> $body:block,
      $($rest:tt)*
    }
  ) => {
    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $doc,
          "\n",
        )?
        "---@param ... ",
        $crate::to_lua_type!($ty),
        $(
          " -- ",
          $param_doc,
        )?
        "\n",
        "---@return ",
        $crate::to_lua_type!($ret),
        "\n",
        "function ",
        constcat::concat![
          $(stringify!($path), "."),+,
          stringify!($func),
        ],
        "(...) end\n\n",
      ])
    }

    lua_annotations!(@table $lua $gw $parent [$($path).+] { $($rest)* });
  };

  (
    @table $lua:ident $gw:ident $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $func:ident = async |$($(#[doc = $param_doc:literal])? $param:ident : $ty:ty),* $(,)?| -> Result<$ret:ty> $body:block,
      $($rest:tt)*
    }
  ) => {
    inventory::submit! {
      $crate::ui::common::lua::lua_macros::LuaAnnotation(constcat::concat![
        $(
          "---",
          $doc,
          "\n",
        )?
        $(
          constcat::concat![
            "---@param ",
            stringify!($param),
            " ",
            $crate::to_lua_type!($ty),
            $(
              " -- ",
              $param_doc,
            )?
            "\n",
          ],
        )*
        "---@return ",
        $crate::to_lua_type!($ret),
        "\n",
        "function ",
        constcat::concat![
          $(stringify!($path), "."),+,
          stringify!($func),
        ],
        "(",
        stringify!($($param),*),
        ") end\n\n",
      ])
    }

    lua_annotations!(@table $lua $gw $parent [$($path).+] { $($rest)* });
  };

  (@table $lua:ident $gw:ident $parent:ident [$($path:ident).+] {}) => {};
}

#[macro_export]
macro_rules! lua {
  (|$gw:ident : $gw_ty:ty| $body:tt) => {
    lua_annotations!($body);

    pub fn create_lua($gw : $gw_ty) -> mlua::Result<mlua::Lua> {
      let lua = mlua::Lua::new();

      let mut globals = lua.globals();

      for add_global in inventory::iter::<super::lua_macros::LuaSetupGlobal> {
        add_global.0(&mut globals)?;
      }

      lua!(@root lua $gw $gw_ty $body);

      Ok(lua)
    }
  };

  (
    @root $lua:ident $gw:ident $gw_ty:ty {
      $(#[doc = $doc:literal])?
      $table:ident = { $($inner:tt)* }
    }
  ) => {
    let $table = $lua.create_table()?;

    lua!(@table $lua $gw ($gw_ty) $table [$table] { $($inner)* });

    $lua.globals().set(stringify!($table), $table)?;
  };

  (
    @table $lua:ident $gw:ident ($gw_ty:ty) $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $table:ident = { $($inner:tt)* },
      $($rest:tt)*
    }
  ) => {
    let $table = $lua.create_table()?;

    lua!(@table $lua $gw ($gw_ty) $table [$($path).+ . $table] { $($inner)* });

    $parent.set(stringify!($table), $table)?;

    lua!(@table $lua $gw ($gw_ty) $parent [$($path).+] { $($rest)* })
  };

  (
    @table $lua:ident $gw:ident ($gw_ty:ty) $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $func:ident = async || -> Result<$ret:ty> $body:block,
      $($rest:tt)*
    }
  ) => {
    {
      #[allow(unused_variables)]
      let $gw = Arc::clone(&$gw);
      $parent.set(stringify!($func), $lua.create_async_function(move |_lua, _: ()| {
        #[allow(unused_variables)]
        let $gw = Arc::clone(&$gw);
        async move {
          ($body) as mlua::Result<$ret>
        }
      })?)?;
    }

    lua!(@table $lua $gw ($gw_ty) $parent [$($path).+] { $($rest)* })
  };

  (
    @table $lua:ident $gw:ident ($gw_ty:ty) $parent:ident [$($path:ident).+] {
      $(#[doc = $doc:literal])?
      $func:ident = async |$($(#[doc = $param_doc:literal])? $param:ident : $ty:ty),* $(,)?| -> Result<$ret:ty> $body:block,
      $($rest:tt)*
    }
  ) => {
    {
      #[allow(unused_variables)]
      let $gw = Arc::clone(&$gw);
      #[allow(unused_parens)]
      $parent.set(stringify!($func), $lua.create_async_function(move |_lua, ($($param),*): ($($ty),*)| {
        #[allow(unused_variables)]
        let $gw = Arc::clone(&$gw);
        async move {
          ($body) as mlua::Result<$ret>
        }
      })?)?;
    }

    lua!(@table $lua $gw ($gw_ty) $parent [$($path).+] { $($rest)* })
  };

  (@table $lua:ident $gw:ident ($gw_ty:ty) $parent:ident [$($path:ident).+] {}) => {};
}
