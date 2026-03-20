use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_true")]
    enabled: bool,
    
    #[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_false")]
    debug: bool,
    
    // Regular boolean field (requires explicit value)
    production: bool,
}

fn main() {
    let kdl_input = r#"
        enabled          // defaults to true
        debug            // defaults to false  
        production #true // explicit value required
    "#;
    
    let config: Config = serde_kdl2::from_str(kdl_input).unwrap();
    
    println!("{:#?}", config);
    // Output:
    // Config {
    //     enabled: true,     // from bare_true default
    //     debug: false,      // from bare_false default
    //     production: true,  // from explicit #true
    // }
    
    // You can still override the defaults with explicit values
    let kdl_override = r#"
        enabled #false   // overrides bare_true default
        debug #true      // overrides bare_false default
        production #false
    "#;
    
    let config2: Config = serde_kdl2::from_str(kdl_override).unwrap();
    println!("{:#?}", config2);
    // Output:
    // Config {
    //     enabled: false,    // explicit override
    //     debug: true,       // explicit override
    //     production: false,
    // }
}