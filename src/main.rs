use clang::{Clang, Entity, EntityKind, TypeKind};
use std::io::{self, Write};

fn main() -> Result<(), io::Error> {
    let clang_path = std::env::var("CLANG_PATH").expect("CLANG_PATH not set");
    let mac_sdk_path = std::env::var("MAC_SDK_PATH").expect("MAC_SDK_PATH not set");
    let appkit_path = format!(
        "{}/System/Library/Frameworks/AppKit.framework/Headers/AppKit.h",
        mac_sdk_path
    );

    let clang = Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, false);

    let translation_unit = index
        .parser(appkit_path)
        .arguments(&[
            "-isysroot",
            mac_sdk_path.as_str(),
            "-I",
            format!("{}/usr/include", mac_sdk_path).as_str(),
            "-I",
            format!("{}/include", clang_path).as_str(),
            "-target",
            "x86_64-apple-macos11.3",
            "-x",
            "objective-c",
        ])
        .parse()
        .unwrap();
    translation_unit.get_diagnostics().iter().for_each(|d| {
        println!("{}", d);
    });

    for child in translation_unit.get_entity().get_children() {
        match child.get_kind() {
            EntityKind::ObjCInterfaceDecl => {
                convert_interface_decl(&child)?;
            }
            EntityKind::ObjCProtocolDecl => {
                // writeln!(file, "{} ObjCProtocolDecl", child.get_name().unwrap())?;
            }
            EntityKind::StructDecl => {
                // writeln!(
                //     file,
                //     "{} StructDecl",
                //     child.get_name().unwrap_or("unknown".to_owned())
                // )?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn convert_interface_decl(e: &Entity) -> io::Result<()> {
    // TODO: platform availability
    let name = e.get_name().unwrap();
    let mut file = std::fs::File::create(format!("appkit/{}.hx", name)).unwrap();
    writeln!(file, "package appkit;")?;
    writeln!(file, "@:objc extern class {} {{", name)?;

    for child in e.get_children() {
        match child.get_kind() {
            EntityKind::ObjCInstanceMethodDecl => {
                let full_name = child.get_name().unwrap();
                let name = (full_name.split(':')).collect::<Vec<_>>()[0];
                let return_type = child.get_result_type().unwrap();
                let return_type_name = convert_type_name(&return_type);
                let arguments = child.get_arguments().unwrap();
                // if arguments.len() > 3 {
                //     dbg!(full_name, child.get_children());
                //     dbg!(child.get_child(0).unwrap().get_type());
                //     std::process::exit(0);
                // }
                write!(file, "    @:native(\"{}\") public function {}(", name, name)?;
                for (i, arg) in arguments.iter().enumerate() {
                    let arg_name = arg.get_name().unwrap();
                    let arg_type = arg.get_type().unwrap();
                    let arg_type_name = convert_type_name(&arg_type);
                    if i == 0 {
                        write!(file, "{}:{}", arg_name, arg_type_name)?;
                    } else {
                        write!(file, " ,{}:{}", arg_name, arg_type_name,)?;
                    }
                }
                writeln!(file, "):{};", return_type_name)?;
                // convert_method_decl(&child, &mut file);
            }
            EntityKind::ObjCClassMethodDecl => {
                let full_name = child.get_name().unwrap();
                let name = (full_name.split(':')).collect::<Vec<_>>()[0];
                let return_type = child.get_result_type().unwrap();
                let return_type_name = convert_type_name(&return_type);
                let arguments = child.get_arguments().unwrap();
                write!(
                    file,
                    "    @:native(\"{}\") public static function {}(",
                    name, name
                )?;
                for (i, arg) in arguments.iter().enumerate() {
                    let arg_name = arg.get_name().unwrap();
                    let arg_type = arg.get_type().unwrap();
                    let arg_type_name = convert_type_name(&arg_type);
                    if i == 0 {
                        write!(file, "{}:{}", arg_name, arg_type_name)?;
                    } else {
                        write!(file, " ,{}:{}", arg_name, arg_type_name,)?;
                    }
                }
                writeln!(file, "):{};", return_type_name)?;
                // convert_method_decl(&child, &mut file);
            }
            EntityKind::ObjCPropertyDecl => {
                // convert_property_decl(&child, &mut file);
            }
            _ => {}
        }
    }
    writeln!(file, "}}")?;
    Ok(())
}

fn convert_type_name(t: &clang::Type) -> String {
    match t.get_kind() {
        TypeKind::ObjCObjectPointer => {
            format!(
                "cpp.Star<{}>",
                convert_type_name(&t.get_pointee_type().unwrap())
            )
        }
        TypeKind::ObjCObject => {
            // println!(
            //     "{} {:?} {:?} {:?}",
            //     t.get_display_name(),
            //     t.get_canonical_type(),
            //     t.get_objc_object_base_type(),
            //     t.get_objc_type_arguments()
            // );
            let type_arguments = t.get_objc_type_arguments();
            if type_arguments.is_empty() {
                convert_type_name(&t.get_objc_object_base_type().unwrap())
            } else {
                format!(
                    "{}<{}>",
                    convert_type_name(&t.get_objc_object_base_type().unwrap()),
                    type_arguments
                        .iter()
                        .map(convert_type_name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        TypeKind::ObjCInterface => {
            // dbg!(t);
            let type_arguments = t.get_objc_type_arguments();
            format!(
                "/* ObjCInterface */ {}",
                if type_arguments.is_empty() {
                    t.get_display_name()
                } else {
                    format!(
                        "{}<{}>",
                        t.get_display_name(),
                        type_arguments
                            .iter()
                            .map(convert_type_name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            )
        }
        TypeKind::ObjCId => "cpp.objc.NSObject".to_owned(),
        TypeKind::ObjCSel => "SEL".to_owned(), //panic!("Sel is not supported, {}", t.get_display_name()),
        TypeKind::BlockPointer => {
            format!(
                "cpp.objc.ObjcBlock<{}>",
                convert_type_name(&t.get_pointee_type().unwrap())
            )
        }
        TypeKind::FunctionPrototype => "haxe.Function".to_owned(),
        TypeKind::Void => "Void".to_owned(),
        k => format!("{} /* {:?} */", t.get_display_name(), k),
    }
}
