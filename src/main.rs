use regex::Regex;
use std::fmt::{Display, Formatter};
use std::fs::{File, read_to_string};
use std::io::Write;

fn main() {
    let str = read_to_string("input").unwrap();
    let v = parse(str.to_string());
    for v in v {
        File::create(format!("{}.java", v.name))
            .unwrap()
            .write_all(v.to_string().as_bytes())
            .unwrap();
    }
}

fn parse(s: String) -> Vec<Class> {
    s.split("\n\n").map(Class::from).collect::<Vec<_>>()
}

struct Class {
    name: String,
    variables: Vec<Variable>,
    methods: Vec<Method>,
}

fn rm_prefix(mut s: String) -> (bool, bool, String) {
    let mut getter = false;
    let mut setter = false;
    if let Some(v) = s.strip_prefix('!') {
        setter = true;
        s = v.to_string();
    } else if let Some(v) = s.strip_prefix('?') {
        getter = true;
        s = v.to_string();
    }

    if let Some(v) = s.strip_prefix('!') {
        setter = true;
        s = v.to_string();
    } else if let Some(v) = s.strip_prefix('?') {
        getter = true;
        s = v.to_string();
    }
    (getter, setter, s)
}

impl From<&str> for Class {
    fn from(value: &str) -> Self {
        let mut methods = vec![];
        let mut variables = vec![];
        let class;
        let lines = value.split('\n').collect::<Vec<_>>();
        if lines.len() > 3 || lines.is_empty() {
            panic!("To much input")
        }
        let cls = lines.first().unwrap();
        if let Some((cls, overr)) = cls.split_once(':') {
            class = cls.to_string();
            for char in overr.chars() {
                match char {
                    'c' => methods.push(Method::OR(Override::Clone)),
                    's' => methods.push(Method::OR(Override::ToString)),
                    'e' => methods.push(Method::OR(Override::Equals)),
                    _ => panic!("unkown char {}", char),
                }
            }
        } else {
            class = cls.to_string();
        }
        let mut parse_meth = |input_str| {
            let pattern_str = r"([+\-#]?)(\w+)\(([^)]*)\)(?::(\w+))?,?";
            let pattern = Regex::new(pattern_str).unwrap();

            for caps in pattern.captures_iter(input_str) {
                let symbol = caps.get(1).map_or("+", |m| m.as_str());
                let function_name = caps.get(2).map(|v| v.as_str()).unwrap();
                let parameters = caps.get(3).map(|v| v.as_str());
                let args = parameters
                    .map(|v| {
                        if v.is_empty() {
                            vec![]
                        } else {
                            v.split(',')
                                .map(|vc| {
                                    let v = vc.split_once(':').unwrap();
                                    Variable {
                                        typ: v.1.trim().to_string(),
                                        name: v.0.trim().to_string(),
                                        getter: false,
                                        setter: false,
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                    })
                    .unwrap_or_default();
                let return_type = caps.get(4).map_or("void", |m| m.as_str());
                methods.insert(
                    0,
                    Method::CM(CustomMethod {
                        visibility: Visibility::from(symbol),
                        return_val: return_type.to_string(),
                        name: function_name.to_string(),
                        args,
                    }),
                );
            }
        };
        let vars = lines.get(1);
        if let Some(v) = vars {
            if v.contains('(') {
                parse_meth(v);
            } else {
                let args = v.split(',');
                for arg in args {
                    let (getter, setter, string) = rm_prefix(arg.to_string());
                    let (name, typ) = string.split_once(':').unwrap();
                    variables.push(Variable {
                        typ: typ.trim().to_string(),
                        name: name.trim().to_string(),
                        getter,
                        setter,
                    });
                }
            }
        }
        let meths = lines.get(2);
        if let Some(input_str) = meths {
            parse_meth(input_str);
        }
        Self {
            name: class,
            variables,
            methods,
        }
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut rows = vec![];
        //class
        rows.push(format!("public class {} {}", self.name, '{'));
        //variables
        rows.append(
            &mut self
                .variables
                .iter()
                .map(|v| leftpad(4, format!("private {} {};", v.typ, v.name)))
                .collect::<Vec<_>>(),
        );
        rows.push("".to_string());
        //constructor
        rows.push(leftpad(4, format!("public {}() {}", self.name, '{')));
        rows.push(leftpad(8, "//TODO: Add default values"));
        rows.push(leftpad(4, "}".to_string()));
        rows.push("".to_string());
        if !self.variables.is_empty() {
            let args = self
                .variables
                .iter()
                .map(|v| format!("{} {}", v.typ, v.name))
                .collect::<Vec<_>>()
                .join(", ");
            rows.push(leftpad(
                4,
                format!("public {}({}) {}", self.name, args, '{'),
            ));
            rows.append(
                &mut self
                    .variables
                    .iter()
                    .map(|v| leftpad(8, format!("set{}({});", some_kind_of_uppercase_first_letter(&v.name), v.name)))
                    .collect::<Vec<_>>(),
            );
            rows.push(leftpad(4, '}'));
            rows.push("".to_string());
        }

        for var in &self.variables {
            if var.setter {
                rows.push(leftpad(
                    4,
                    format!(
                        "public void set{}({} {}) {}",
                        some_kind_of_uppercase_first_letter(&var.name),
                        var.typ,
                        var.name,
                        '{'
                    ),
                ));
                rows.push(leftpad(8, format!("this.{} = {};", var.name, var.name)));
                rows.push(leftpad(4, "}"));
                rows.push("".to_string());
            }

            if var.getter {
                rows.push(leftpad(
                    4,
                    format!(
                        "public {} {}() {}",
                        var.typ,
                        &var.name,
                        '{'
                    ),
                ));
                rows.push(leftpad(8, format!("return this.{};", var.name)));
                rows.push(leftpad(4, "}"));
                rows.push("".to_string());
            }
        }

        for method in &self.methods {
            match method {
                Method::CM(v) => {
                    rows.push(leftpad(
                        4,
                        format!(
                            "{} {} {}({}) {}",
                            v.visibility.clone(),
                            v.return_val,
                            v.name,
                            v.args
                                .iter()
                                .map(|v| format!("{} {}", v.typ, v.name))
                                .collect::<Vec<_>>()
                                .join(", "),
                            '{'
                        ),
                    ));
                    rows.push(leftpad(8, "//TODO: implement body"));
                    rows.push(leftpad(4, "}"));
                }
                Method::OR(v) => rows.push(v.to_string(&self.name, &self.variables)),
            }
            rows.push("".to_string());
        }

        rows.push("}".to_string());
        write!(f, "{}", rows.join("\n"))
    }
}

struct Variable {
    typ: String,
    name: String,
    getter: bool,
    setter: bool,
}

impl Variable {
    fn check_if_equals_needed(&self) -> bool {
        self.typ
            .chars()
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .is_uppercase()
    }
}

enum Method {
    CM(CustomMethod),
    OR(Override),
}

struct CustomMethod {
    visibility: Visibility,
    return_val: String,
    name: String,
    args: Vec<Variable>,
}

#[derive(Clone)]
enum Visibility {
    Public,
    Protected,
    Private,
}

impl From<&str> for Visibility {
    fn from(value: &str) -> Self {
        match value {
            "+" | "" => Visibility::Public,
            "#" => Visibility::Protected,
            "-" => Visibility::Private,
            &_ => panic!(),
        }
    }
}


impl Display for Visibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Protected => write!(f, "protected"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

enum Override {
    Clone,
    Equals,
    ToString,
}

impl Override {
    fn to_string(&self, obj: &str, vars: &[Variable]) -> String {
        let mut lines = vec![];
        lines.push(leftpad(4, "@Override"));
        match self {
            Override::Clone => {
                lines.push(leftpad(4, "public Object clone() {"));
                lines.push(leftpad(
                    8,
                    format!(
                        "return new {}({});",
                        obj,
                        vars.iter()
                            .map(|v| match v.check_if_equals_needed() {
                                true => format!("({}){}().clone()", v.typ, &v.name),
                                false => format!("{}()", &v.name),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                lines.push(leftpad(4, "}"));
            }
            Override::Equals => {
                lines.push(leftpad(4, "public boolean equals(Object o) {"));
                lines.push(leftpad(8, "if (o == this) return true;"));
                lines.push(leftpad(
                    8,
                    format!("if (!(o instanceof {})) return false;", obj),
                ));
                lines.push(leftpad(8, format!("{} other = ({})o;", obj, obj)));
                let check = vars
                    .iter()
                    .map(|v| {
                        format!(
                            "this.{}(){}",
                            &v.name,
                            match v.check_if_equals_needed() {
                                true => format!(".equals(other.{}())", &v.name),
                                false => format!(" == other.{}()", &v.name),
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" && ");
                lines.push(leftpad(8, format!("return {};", check)));
                lines.push(leftpad(4, "}"));
            }
            Override::ToString => {
                lines.push(leftpad(4, "public String toString() {"));
                lines.push(leftpad(
                    8,
                    "return String.format(\"%s\", super.toString()); //TODO: replace",
                ));
                lines.push(leftpad(4, "}"))
            }
        }
        lines.join("\n")
    }
}

fn leftpad(len: usize, s: impl ToString) -> String {
    format!("{}{}", vec![" "; len].join(""), s.to_string())
}

fn some_kind_of_uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
