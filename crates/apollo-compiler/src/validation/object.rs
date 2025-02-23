use crate::{
    ast,
    validation::diagnostics::{DiagnosticData, ValidationError},
    ValidationDatabase,
};
use std::collections::HashSet;

pub(crate) fn validate_object_type_definitions(
    db: &dyn ValidationDatabase,
) -> Vec<ValidationError> {
    let mut diagnostics = vec![];

    let defs = &db.ast_types().objects;
    for def in defs.values() {
        diagnostics.extend(db.validate_object_type_definition(def.clone()))
    }

    diagnostics
}

pub(crate) fn validate_object_type_definition(
    db: &dyn ValidationDatabase,
    object: ast::TypeWithExtensions<ast::ObjectTypeDefinition>,
) -> Vec<ValidationError> {
    let mut diagnostics = Vec::new();

    let schema = db.schema();

    diagnostics.extend(super::directive::validate_directives(
        db,
        object.directives(),
        ast::DirectiveLocation::Object,
        // objects don't use variables
        Default::default(),
    ));

    // Collect all fields, including duplicates
    let field_definitions: Vec<_> = object.fields().cloned().collect();
    let field_names: HashSet<_> = field_definitions
        .iter()
        .map(|field| field.name.clone())
        .collect();

    // Object Type field validations.
    diagnostics.extend(db.validate_field_definitions(field_definitions));

    // Implements Interfaces validation.
    let implements_interfaces: Vec<_> = object.implements_interfaces().cloned().collect();
    diagnostics.extend(super::interface::validate_implements_interfaces(
        db,
        &object.definition.clone().into(),
        &implements_interfaces,
    ));

    // When defining an interface that implements another interface, the
    // implementing interface must define each field that is specified by
    // the implemented interface.
    //
    // Returns a Missing Field error.
    for implements_interface in object.implements_interfaces() {
        if let Some(interface) = schema.get_interface(implements_interface) {
            for interface_field in interface.fields.values() {
                if field_names.contains(&interface_field.name) {
                    continue;
                }

                diagnostics.push(ValidationError::new(
                    object.definition.location(),
                    DiagnosticData::MissingInterfaceField {
                        name: object.definition.name.clone(),
                        implements_location: implements_interface.location(),
                        interface: implements_interface.clone(),
                        field: interface_field.name.clone(),
                        field_location: interface_field.location(),
                    },
                ));
            }
        }
    }

    diagnostics
}
