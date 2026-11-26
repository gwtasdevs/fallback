use std::{path::Path, sync::Arc};

use anyhow::{Result, anyhow};
use indoc::indoc;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::gw::Gw;

mod annotations;
mod gw;
pub mod lua_macros;
mod models;

pub fn init_lua(gw: Arc<Gw>) -> mlua::Lua {
  let _ = annotations::write_annotations();
  gw::create_lua(gw).unwrap()
}

pub fn execute_script(lua: &mlua::Lua, path: &Path, code: &str) -> Result<mlua::Table> {
  let globals = lua.globals();
  let env = lua.create_table()?;
  let meta = lua.create_table()?;
  meta.set("__index", globals)?;
  env.set_metatable(Some(meta))?;

  lua
    .load(code)
    .set_name(format!("@{}", path.display()))
    .set_environment(env.clone())
    .exec()?;

  Ok(env)
}

pub fn update_lua_values<F: Fn(&str) -> Result<String>>(
  mut contents: String,
  root_name: &str,
  field_names: &[&str],
  generate_update: F,
) -> Result<String> {
  // TODO: allow generate_update to return fields that should be omitted - and removed if they are already present.
  const ROOT_QUERY: &str = indoc!(
    r#"
    (chunk
      (assignment_statement
        (variable_list (identifier) @var_name)
        operator: "="
        (expression_list (table_constructor) @table_node)
        (#eq? @var_name "{}")
      )
    )
  "#
  );

  const TABLE_QUERY: &str = indoc!(
    r#"
    (field
      name: (identifier) @field_name
      value: (_) @field_value
      (#any-of? @field_name {})
    )
  "#
  );

  let mut parser = Parser::new();
  parser.set_language(&tree_sitter_lua::LANGUAGE.into())?;
  let tree = parser.parse(&contents, None).ok_or(anyhow!("Could not parse file."))?;

  let root_query = ROOT_QUERY.replace("{}", root_name);
  let root_query = Query::new(&tree_sitter_lua::LANGUAGE.into(), &root_query)?;
  let table_node_index = root_query
    .capture_index_for_name("table_node")
    .ok_or(anyhow!("Could not find 'table_node' in query."))?;
  let mut cursor = QueryCursor::new();
  let mut query_matches = cursor.matches(&root_query, tree.root_node(), contents.as_bytes());

  let table_query = TABLE_QUERY.replace(
    "{}",
    &field_names
      .iter()
      .map(|name| format!(r#""{}""#, name))
      .collect::<Vec<_>>()
      .join(" "),
  );
  let table_query = Query::new(&tree_sitter_lua::LANGUAGE.into(), &table_query)?;
  let field_name_index = table_query
    .capture_index_for_name("field_name")
    .ok_or(anyhow!("Could not find 'field_name' in query."))?;
  let field_value_index = table_query
    .capture_index_for_name("field_value")
    .ok_or(anyhow!("Could not find 'field value' in query."))?;

  let mut updates: Vec<(core::ops::Range<usize>, String, String)> = vec![];

  if let Some(query_match) = query_matches.next()
    && let Some(capture) = query_match
      .captures()
      .iter()
      .find(|capture| capture.index == table_node_index)
  {
    let table_node = capture.node;

    let mut cursor = table_node.walk();
    let first_field = table_node.children(&mut cursor).find(|child| child.kind() == "field");
    let indentation = first_field
      .map(|node| {
        let col = node.start_position().column;
        let indent_end = node.start_byte();
        let indent_start = indent_end - col;

        contents[indent_start..indent_end].to_string()
      })
      .unwrap_or("  ".to_string());

    let mut field_cursor = QueryCursor::new();
    let mut field_matches = field_cursor.matches(&table_query, table_node, contents.as_bytes());

    while let Some(field_match) = field_matches.next() {
      let field_name_capture = field_match
        .captures()
        .iter()
        .find(|capture| capture.index == field_name_index);
      let field_value_capture = field_match
        .captures()
        .iter()
        .find(|capture| capture.index == field_value_index);
      if let Some(field_name) = field_name_capture
        && let Some(field_value) = field_value_capture
      {
        let field_name = std::str::from_utf8(&contents.as_bytes()[field_name.node.byte_range()])?;
        let updated = format_update(generate_update(field_name)?, &indentation);
        updates.push((field_value.node.byte_range(), field_name.to_string(), updated));
      }
    }

    // append any nodes missing from the lua file
    let missing_fields: Vec<_> = field_names
      .iter()
      .filter(|&field_name| !updates.iter().any(|(_, name, _)| name == field_name))
      .collect();

    if !missing_fields.is_empty() {
      let penultimate_node = table_node
        .child(table_node.child_count() - 2)
        .ok_or(anyhow!("Could not find closing bracket of table node."))?;

      if penultimate_node.kind() == "field" {
        // the penultimate node is a field, so we need to add a field separator
        let pos = penultimate_node.end_byte();
        updates.push((pos..pos, "field_separator".to_string(), ",".to_string()));
      }

      let table_closing_bracket = table_node
        .child(table_node.child_count() - 1)
        .ok_or(anyhow!("Could not find closing bracket of table node."))?;

      let pos = table_closing_bracket.start_byte();
      for field_name in missing_fields {
        let updated = format_update(generate_update(field_name)?, &indentation);
        updates.push((
          pos..pos,
          field_name.to_string(),
          format!("{}{} = {},\n", indentation, field_name, updated),
        ));
      }
    }
  }

  updates.sort_by_key(|(range, _name, _text)| range.start);
  updates.reverse();
  for (range, _name, text) in updates {
    contents.replace_range(range, &text);
  }

  Ok(contents)
}

fn format_update(update: String, indentation: &str) -> String {
  update
    .trim()
    .lines()
    .enumerate()
    .map(|(i, line)| {
      if i == 0 {
        line.to_string()
      } else {
        format!("{}{}", indentation, line)
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod tests {
  use anyhow::bail;

  use super::*;

  #[test]
  fn test_script_parse_and_update() {
    let contents = indoc!(
      r#"
      ---@type Script
      script = {
        name = "Test",
        triggers = {
          {
            type = "tick",
          },
        },
      }
    "#
    )
    .to_string();

    let result = update_lua_values(
      contents,
      "script",
      &["name", "throttle", "triggers"],
      |field_name| match field_name {
        "name" => Ok(r#""changed""#.to_string()),
        "throttle" => Ok(r#"nil"#.to_string()),
        "triggers" => Ok(
          indoc!(
            r#"
          {
            { type = "map", map_id = 1 }
          }
        "#
          )
          .to_string(),
        ),
        _ => bail!("Invalid field name {}", field_name),
      },
    );

    let expected = indoc!(
      r#"
      ---@type Script
      script = {
        name = "changed",
        triggers = {
          { type = "map", map_id = 1 }
        },
        throttle = nil,
      }
    "#
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
  }
}
