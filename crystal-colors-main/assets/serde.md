# Serde
- serializing and deserializing
- serde is not parsing libary
    - serde connect you to the serializing and deserializing trait implementation.
    - serde does not produce a new data format
- very fast, allmost zero cost
- serde data model: "use `this`data format and turn it into `this`data type"
- use serde::{Serialize, Deserialize}
- #[derive(Serialize, Deserialize)]

## Serialize
- Change objects or datastructure into a format to save or send (JSON...)

## Deserialize
- Change the dataform back into the needed form
- Visiter visits every element in my struct

## atributes: 
- #[serde(rename)]
- #[serde(rename_all)]
- #[serde(deny_unknown_fields)]
- #[serde(tag = "type")]
- #[serde(bound)]
    - copy- paste the text you type in
- #[serde(default)]
    - if this field is not in the input, set the value to default instead of returning an error
- #[serde(transparent)]
    - dont call structs with just one element. It strait calls serialize and deserialize on the single inner value. (Works just for new type structs)
- #[serde(from = "FromType")]
    - example:
        - #[derive(Serialize, Deserialize)]
        - struct Foo {
        -     a: u64,
        -     #[serde(from = "String")]
        -     b: Foo,
        - }
    - Deserialize into a String and than use the from trait to turn it into a Foo
## Enum
- variant attributes
    - #[serde(borrow)]

