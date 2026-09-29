use super::model::{ConditionalBranch, EventArgument, EventBinding};
use super::{
    AttributeValue, ComponentRegistry, Expression, HydrationPolicy, Scope, ServiceDefinition,
    ServiceRegistry, TemplateAttribute, TemplateNode, TemplateNodeKind, TemplateProgram, TextPart,
    is_trivia,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use tachyon_contracts::{CountedComparison, ViewSourceMap};

#[derive(Clone, Debug)]
pub(crate) struct ClientRenderedView {
    pub(crate) html: String,
    pub(crate) source_map: ViewSourceMap,
    pub(crate) components: BTreeSet<String>,
    pub(crate) page_bindings: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientViewRenderer<'a> {
    components: &'a ComponentRegistry,
    services: &'a ServiceRegistry,
    shared_data: &'a BTreeSet<String>,
}

impl<'a> ClientViewRenderer<'a> {
    const SHARED_DATA_ROOT: &'static str = "data";
    pub(crate) const VIEW_ROOTS: [&'static str; 2] = [Self::SHARED_DATA_ROOT, "page"];
    const MAX_CLIENT_PLAN_BYTES: usize = 10 * 1_024 * 1_024;
    pub(crate) const RUNTIME: &'static str = concat!(
        include_str!("application-events.js"),
        "\n",
        include_str!("tac-storage.js"),
        "\n",
        include_str!("tac-web.js"),
        "\n",
        include_str!("tac-client.js")
    );

    pub(crate) const fn new(
        components: &'a ComponentRegistry,
        services: &'a ServiceRegistry,
        shared_data: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            components,
            services,
            shared_data,
        }
    }

    pub(crate) fn render(
        self,
        program: &TemplateProgram,
        output_path: &str,
        initial_state: &Scope,
        route: &str,
    ) -> Result<ClientRenderedView, Failure> {
        let mut encoder = Encoder::new(self.components, self.services);
        let catalog = self.service_catalog();
        let nodes = encoder.nodes(&program.nodes)?;
        self.check_shared_data(program, &nodes)?;
        let plan = json!({
            "schemaVersion": 1,
            "route": route,
            "state": initial_state,
            "services": catalog,
            "nodes": nodes,
        });
        let plan = Self::serialized_plan(program, &plan)?;
        let source_map = ViewSourceMap::v1(
            String::from(output_path),
            vec![program.source_path.clone()],
            Vec::new(),
        );
        Ok(ClientRenderedView {
            html: page_html(&plan),
            source_map,
            components: encoder.used_components,
            page_bindings: encoder.page_bindings,
        })
    }

    pub(crate) fn element(self, name: &str) -> Result<Value, Failure> {
        let mut encoder = Encoder::new(self.components, self.services);
        let mut element = encoder.exported_component(name)?;
        element["services"] = Value::Object(self.service_catalog());
        Ok(element)
    }

    fn service_catalog(&self) -> serde_json::Map<String, Value> {
        let mut catalog = serde_json::Map::new();
        for (instance, service) in self.services.iter() {
            let mut entry = json!({
                "module": service.module_url(),
                "scope": service_scope(service),
                "native": service.linked(),
            });
            add_required(&mut entry, service);
            catalog.insert(instance.clone(), entry);
        }
        catalog
    }

    fn check_shared_data(&self, program: &TemplateProgram, nodes: &[Value]) -> Result<(), Failure> {
        let mut read = BTreeSet::new();
        for node in nodes {
            data_references(node, &mut read);
        }
        for name in &read {
            if !self.shared_data.contains(name) {
                return Err(missing_data_failure(program, name));
            }
        }
        Ok(())
    }

    fn serialized_plan(program: &TemplateProgram, plan: &Value) -> Result<String, Failure> {
        let plan =
            serde_json::to_string(plan).map_err(|error| serialization_failure(program, &error))?;
        if plan.len() > Self::MAX_CLIENT_PLAN_BYTES {
            return Err(Failure::one(diagnostic(
                1305,
                "Tac client render plan exceeds the 10 MiB limit.",
                Some(String::from(
                    "Split the view into fewer or smaller components.",
                )),
                source_span(&program.source_path, 0, 0),
            )));
        }
        Ok(plan.replace('<', "\\u003c"))
    }
}

struct EnclosingComponent {
    name: String,
    props: BTreeSet<String>,
    services: Vec<String>,
}

struct Encoder<'a> {
    components: &'a ComponentRegistry,
    services: &'a ServiceRegistry,
    enclosing: Vec<EnclosingComponent>,
    loop_bindings: Vec<String>,
    used_components: BTreeSet<String>,
    component_owner_depth: usize,
    page_bindings: bool,
    node_count: usize,
}

impl<'a> Encoder<'a> {
    const MAX_CLIENT_PLAN_NODES: usize = 100_000;

    const fn new(components: &'a ComponentRegistry, services: &'a ServiceRegistry) -> Self {
        Self {
            components,
            services,
            enclosing: Vec::new(),
            loop_bindings: Vec::new(),
            used_components: BTreeSet::new(),
            component_owner_depth: 0,
            page_bindings: false,
            node_count: 0,
        }
    }

    fn nodes(&mut self, nodes: &[TemplateNode]) -> Result<Vec<Value>, Failure> {
        nodes.iter().map(|node| self.node(node)).collect()
    }

    fn node(&mut self, node: &TemplateNode) -> Result<Value, Failure> {
        self.count_node(node)?;
        match &node.kind {
            TemplateNodeKind::Element {
                tag,
                attributes,
                children,
                void,
            } => self.element(tag, attributes, children, *void),
            TemplateNodeKind::Text(parts) => text_value(parts),
            TemplateNodeKind::Comment(value) => Ok(json!({ "k": "comment", "value": value })),
            TemplateNodeKind::Conditional {
                condition,
                branch,
                children,
            } => self.conditional(condition.as_ref(), *branch, children),
            TemplateNodeKind::Iteration {
                binding,
                iterable,
                children,
            } => self.iteration(binding, iterable, children),
            TemplateNodeKind::CountedIteration {
                binding,
                from,
                comparison,
                to,
                step,
                children,
            } => self.counted_iteration(binding, from, *comparison, to, step, children),
            TemplateNodeKind::Switch { value, children } => self.switch(value, children),
            TemplateNodeKind::Case { when, children } => self.case(when.as_ref(), children),
            TemplateNodeKind::Component {
                name,
                properties,
                hydrate,
                children,
            } => self.component(node, name, properties, *hydrate, children),
            TemplateNodeKind::Slot => Ok(json!({ "k": "slot" })),
        }
    }

    fn count_node(&mut self, node: &TemplateNode) -> Result<(), Failure> {
        self.node_count = self.node_count.saturating_add(1);
        if self.node_count > Self::MAX_CLIENT_PLAN_NODES {
            return Err(client_failure(
                node,
                1305,
                "Tac client render plan exceeds 100,000 nodes.",
            ));
        }
        Ok(())
    }

    fn element(
        &mut self,
        tag: &str,
        attributes: &BTreeMap<String, TemplateAttribute>,
        children: &[TemplateNode],
        void: bool,
    ) -> Result<Value, Failure> {
        Ok(json!({
            "k": "element",
            "tag": tag,
            "attributes": self.attributes(attributes)?,
            "children": self.nodes(children)?,
            "void": void,
        }))
    }

    fn conditional(
        &mut self,
        condition: Option<&Expression>,
        branch: ConditionalBranch,
        children: &[TemplateNode],
    ) -> Result<Value, Failure> {
        Ok(json!({
            "k": "conditional",
            "branch": branch_name(branch),
            "condition": condition.map(expression_value).transpose()?,
            "children": self.nodes(children)?,
        }))
    }

    fn iteration(
        &mut self,
        binding: &str,
        iterable: &Expression,
        children: &[TemplateNode],
    ) -> Result<Value, Failure> {
        Ok(json!({
            "k": "iteration",
            "binding": binding,
            "iterable": expression_value(iterable)?,
            "children": self.loop_body(binding, children)?,
        }))
    }

    fn counted_iteration(
        &mut self,
        binding: &str,
        from: &Expression,
        comparison: CountedComparison,
        to: &Expression,
        step: &Expression,
        children: &[TemplateNode],
    ) -> Result<Value, Failure> {
        Ok(json!({
            "k": "counted",
            "binding": binding,
            "from": expression_value(from)?,
            "comparison": comparison,
            "to": expression_value(to)?,
            "step": expression_value(step)?,
            "children": self.loop_body(binding, children)?,
        }))
    }

    fn loop_body(
        &mut self,
        binding: &str,
        children: &[TemplateNode],
    ) -> Result<Vec<Value>, Failure> {
        let outer = self.loop_bindings.len();
        let names = binding
            .split(|character: char| !(character.is_alphanumeric() || character == '_'))
            .filter(|name| !name.is_empty())
            .map(String::from);
        self.loop_bindings.extend(names);
        let body = self.nodes(children);
        self.loop_bindings.truncate(outer);
        body
    }

    fn switch(&mut self, value: &Expression, children: &[TemplateNode]) -> Result<Value, Failure> {
        Ok(json!({
            "k": "switch",
            "value": expression_value(value)?,
            "children": self.nodes(children)?,
        }))
    }

    fn case(
        &mut self,
        when: Option<&Expression>,
        children: &[TemplateNode],
    ) -> Result<Value, Failure> {
        Ok(json!({
            "k": "case",
            "when": when.map(expression_value).transpose()?,
            "children": self.nodes(children)?,
        }))
    }

    fn component(
        &mut self,
        node: &TemplateNode,
        name: &str,
        properties: &BTreeMap<String, TemplateAttribute>,
        declared_policy: Option<HydrationPolicy>,
        children: &[TemplateNode],
    ) -> Result<Value, Failure> {
        let component = self.components.get(name).ok_or_else(|| {
            client_failure(node, 1402, &format!("Unknown Tac component '<{name}>'."))
        })?;
        let template_nodes = &component.program().nodes;
        let has_children = children.iter().any(|child| !is_trivia(child));
        if has_children && !contains_slot(template_nodes) {
            return Err(client_failure(
                node,
                1404,
                &format!("Component '<{name}>' received children but declares no <slot>."),
            ));
        }
        let service_names = self.named_services(template_nodes);
        let has_services = !service_names.is_empty();
        let policy = mount_policy(node, name, declared_policy, has_services)?;
        let active = is_active(policy);
        if active {
            self.used_components.insert(String::from(name));
        }
        if active {
            self.check_required(node, name, properties, &service_names)?;
            self.check_injected(node, name, properties, &service_names)?;
        }
        let services = self.service_entries(&service_names);
        let encoded_properties = self.attributes(properties)?;
        self.enclosing.push(EnclosingComponent {
            name: String::from(name),
            props: properties.keys().cloned().collect(),
            services: service_names,
        });
        let template = self.component_template(template_nodes, active);
        self.enclosing.pop();
        let template = template?;
        let slot = self.nodes(children)?;
        Ok(json!({
            "k": "component",
            "name": name,
            "properties": encoded_properties,
            "mount": policy.map(HydrationPolicy::name),
            "services": services,
            "scope": component.style_path().is_some(),
            "template": template,
            "slot": slot,
        }))
    }

    fn exported_component(&mut self, name: &str) -> Result<Value, Failure> {
        let Some(component) = self.components.get(name) else {
            return Ok(Value::Null);
        };
        let template_nodes = &component.program().nodes;
        let service_names = self.named_services(template_nodes);
        let props = self.element_props(template_nodes, &service_names);
        let active = !service_names.is_empty();
        let services = self.service_entries(&service_names);
        self.enclosing.push(EnclosingComponent {
            name: String::from(name),
            props: props.clone(),
            services: service_names,
        });
        let template = self.component_template(template_nodes, active);
        self.enclosing.pop();
        let slot = if contains_slot(template_nodes) {
            vec![
                json!({ "k": "element", "tag": "slot", "attributes": [], "children": [], "void": false }),
            ]
        } else {
            Vec::new()
        };
        Ok(json!({
            "props": props,
            "node": {
                "k": "component",
                "name": name,
                "exported": true,
                "properties": [],
                "mount": active.then_some(HydrationPolicy::Load.name()),
                "services": services,
                "scope": component.style_path().is_some(),
                "template": template?,
                "slot": slot,
            },
        }))
    }

    fn element_props(
        &self,
        template_nodes: &[TemplateNode],
        service_names: &[String],
    ) -> BTreeSet<String> {
        let mut props = BTreeSet::new();
        collect_identifiers(template_nodes, &mut props);
        for service in service_names
            .iter()
            .filter_map(|name| self.services.get(name))
        {
            props.extend(service.required().iter().cloned());
        }
        props.retain(|prop| {
            !service_names.contains(prop)
                && !ClientViewRenderer::VIEW_ROOTS.contains(&prop.as_str())
        });
        props
    }

    fn check_required(
        &self,
        node: &TemplateNode,
        name: &str,
        properties: &BTreeMap<String, TemplateAttribute>,
        service_names: &[String],
    ) -> Result<(), Failure> {
        for instance in service_names {
            let Some(service) = self.services.get(instance) else {
                continue;
            };
            let required = service.required();
            if required.is_empty() {
                continue;
            }
            let class = service.class();
            if !service.is_prototype() {
                let fields = required
                    .iter()
                    .map(|field| format!("`{field}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let message = format!(
                    "{class} is a singleton, so it gets no props, but NeedsAll requires {fields}. \
                     Give those fields initial values or remove the singleton lifetime."
                );
                return Err(client_failure(node, 1407, &message));
            }
            for field in required {
                let Some(attribute) = properties.get(field) else {
                    let from = self
                        .enclosing
                        .last()
                        .map(|enclosing| format!(" (from <{}>)", enclosing.name))
                        .unwrap_or_default();
                    let message = format!(
                        "{class} needs `{field}`; pass :{field}=\"...\" to <{name}>{from}."
                    );
                    return Err(client_failure(node, 1407, &message));
                };
                self.check_forwarded(node, name, class, field, attribute)?;
            }
        }
        Ok(())
    }

    fn check_injected(
        &self,
        node: &TemplateNode,
        name: &str,
        properties: &BTreeMap<String, TemplateAttribute>,
        service_names: &[String],
    ) -> Result<(), Failure> {
        let receivers = service_names
            .iter()
            .filter_map(|instance| self.services.get(instance))
            .collect::<Vec<_>>();
        for property in properties.keys() {
            let receiver = Self::injected_receiver(&receivers, property);
            receiver.map_or(Ok(()), |receiver| {
                Err(self.injected_failure(node, name, receiver, property))
            })?;
        }
        Ok(())
    }

    fn injected_receiver<'b>(
        receivers: &[&'b ServiceDefinition],
        property: &str,
    ) -> Option<&'b str> {
        let required = receivers
            .iter()
            .any(|service| service.required().iter().any(|field| field == property));
        let injected = receivers
            .iter()
            .find(|service| service.injected().iter().any(|field| field == property));
        injected
            .filter(|_| !required)
            .map(|service| service.class())
    }

    fn injected_failure(
        &self,
        node: &TemplateNode,
        name: &str,
        receiver: &str,
        property: &str,
    ) -> Failure {
        let message = match self.services.get(property) {
            Some(service) => format!(
                "`{property}` is both a property that <{name}> passes and the name of {}'s instance, \
                 so {receiver} gets the service and the property never arrives. Rename the field so \
                 that it receives the property, or mark it @Needs and stop passing :{property} if \
                 you meant the service.",
                service.class()
            ),
            None => format!(
                "`{property}` is both a property that <{name}> passes and a field that {receiver} \
                 gets injected, so the property never arrives. Rename the field so that it receives \
                 the property, or stop passing :{property}."
            ),
        };
        client_failure(node, 1407, &message)
    }

    fn check_forwarded(
        &self,
        node: &TemplateNode,
        name: &str,
        class: &str,
        field: &str,
        attribute: &TemplateAttribute,
    ) -> Result<(), Failure> {
        let AttributeValue::Dynamic(expression) = &attribute.value else {
            return Ok(());
        };
        let Some(enclosing) = self.enclosing.last() else {
            return Ok(());
        };
        let source = expression.source().trim();
        let bare = source
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
            && source
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_');
        let known = enclosing.props.contains(source)
            || enclosing.services.iter().any(|service| service == source)
            || self.loop_bindings.iter().any(|binding| binding == source)
            || ClientViewRenderer::VIEW_ROOTS.contains(&source);
        if !bare || known {
            return Ok(());
        }
        let outer = &enclosing.name;
        let message = format!(
            "{class} needs `{field}`; <{name}> gets it from `{source}`, but <{outer}> is not given `{source}`; \
             pass :{source}=\"...\" to <{outer}>."
        );
        Err(client_failure(node, 1407, &message))
    }

    fn named_services(&self, template_nodes: &[TemplateNode]) -> Vec<String> {
        let mut named = BTreeSet::new();
        collect_identifiers(template_nodes, &mut named);
        self.services
            .names()
            .into_iter()
            .filter(|instance| named.contains(instance))
            .collect()
    }

    fn service_entries(&self, service_names: &[String]) -> Vec<Value> {
        let mut entries = Vec::new();
        for instance in service_names {
            let Some(service) = self.services.get(instance) else {
                continue;
            };
            let mut entry = json!({
                "name": instance,
                "module": service.module_url(),
                "scope": service_scope(service),
                "native": service.linked(),
            });
            add_required(&mut entry, service);
            entries.push(entry);
        }
        entries
    }

    fn component_template(
        &mut self,
        template_nodes: &[TemplateNode],
        active: bool,
    ) -> Result<Vec<Value>, Failure> {
        self.component_owner_depth += usize::from(active);
        let template = self.nodes(template_nodes);
        self.component_owner_depth -= usize::from(active);
        template
    }

    fn attributes(
        &mut self,
        attributes: &BTreeMap<String, TemplateAttribute>,
    ) -> Result<Vec<Value>, Failure> {
        let mut encoded = Vec::new();
        for (name, attribute) in attributes {
            encoded.push(self.attribute(name, attribute)?);
        }
        Ok(encoded)
    }

    fn attribute(&mut self, name: &str, attribute: &TemplateAttribute) -> Result<Value, Failure> {
        match &attribute.value {
            AttributeValue::Static(value) | AttributeValue::Control(value) => {
                Ok(json!({ "name": name, "value": value }))
            }
            AttributeValue::Dynamic(expression) => Ok(json!({
                "name": name,
                "expression": expression_value(expression)?,
            })),
            AttributeValue::Event(binding) => {
                let event_type = name
                    .strip_prefix("data-tac-on-")
                    .unwrap_or(name)
                    .replace("__", ":");
                self.page_bindings |= self.component_owner_depth == 0;
                Ok(json!({ "name": name, "eventType": event_type, "event": event_value(binding)? }))
            }
        }
    }
}

fn page_html(plan: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <script id=\"tachyon-view\" type=\"application/json\" data-tachyon-runtime>{plan}</script>\
         <script type=\"module\" src=\"/.tachyon/tac-client.js\" data-tachyon-runtime></script>\
         </head><body><noscript>Tac requires JavaScript to render this view.</noscript></body></html>"
    )
}

fn mount_policy(
    node: &TemplateNode,
    name: &str,
    declared_policy: Option<HydrationPolicy>,
    has_services: bool,
) -> Result<Option<HydrationPolicy>, Failure> {
    let policy = declared_policy.or_else(|| has_services.then_some(HydrationPolicy::Load));
    if is_active(policy) && !has_services {
        return Err(client_failure(
            node,
            1405,
            &format!("Client component '<{name}>' declares a mount policy but names no service."),
        ));
    }
    Ok(policy)
}

fn is_active(policy: Option<HydrationPolicy>) -> bool {
    policy.is_some_and(|value| value != HydrationPolicy::Never)
}

fn add_required(entry: &mut Value, service: &ServiceDefinition) {
    if service.compiled() || !service.required().is_empty() {
        entry["required"] = json!(service.required());
    }
    if !service.stored().is_empty() {
        entry["stored"] = json!(service.stored());
    }
}

fn service_scope(service: &ServiceDefinition) -> &'static str {
    if service.is_prototype() {
        "prototype"
    } else {
        "singleton"
    }
}

const fn branch_name(branch: ConditionalBranch) -> &'static str {
    match branch {
        ConditionalBranch::If => "if",
        ConditionalBranch::ElseIf => "else-if",
        ConditionalBranch::Else => "else",
    }
}

fn text_value(parts: &[TextPart]) -> Result<Value, Failure> {
    let parts = parts.iter().map(text_part).collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "k": "text", "parts": parts }))
}

fn text_part(part: &TextPart) -> Result<Value, Failure> {
    match part {
        TextPart::Literal(value, _) => Ok(json!({ "value": value })),
        TextPart::Interpolation(expression, _) => {
            Ok(json!({ "expression": expression_value(expression)? }))
        }
    }
}

fn event_value(binding: &EventBinding) -> Result<Value, Failure> {
    let arguments = binding
        .arguments
        .iter()
        .map(event_argument)
        .collect::<Result<Vec<_>, _>>()?;
    let assign = binding.assign.as_ref().map(|assignment| {
        json!({
            "target": assignment.target,
            "operator": assignment.operator,
        })
    });
    Ok(json!({
        "handler": binding.handler,
        "arguments": arguments,
        "assign": assign,
    }))
}

fn event_argument(argument: &EventArgument) -> Result<Value, Failure> {
    match argument {
        EventArgument::Literal(value) => serde_json::from_str::<Value>(value)
            .map(|value| json!({ "value": value }))
            .map_err(|error| Failure::one(diagnostic(1306, error.to_string(), None, None))),
        EventArgument::EventPath(path) => Ok(json!({ "event": path })),
        EventArgument::Scope(expression) => {
            Ok(json!({ "expression": expression_value(expression)? }))
        }
    }
}

fn expression_value(expression: &Expression) -> Result<Value, Failure> {
    serde_json::from_str(&expression.to_client_json()).map_err(|error| {
        Failure::one(diagnostic(
            1305,
            format!("Cannot serialize a Tac client expression: {error}"),
            None,
            None,
        ))
    })
}

fn collect_identifiers(nodes: &[TemplateNode], names: &mut BTreeSet<String>) {
    collect_names_in_scope(nodes, &BTreeSet::new(), names);
}

fn collect_names_in_scope(
    nodes: &[TemplateNode],
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    for node in nodes {
        collect_node_names(node, loop_bindings, names);
    }
}

fn collect_node_names(
    node: &TemplateNode,
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    match &node.kind {
        TemplateNodeKind::Element { attributes, .. }
        | TemplateNodeKind::Component {
            properties: attributes,
            ..
        } => collect_attribute_names(attributes, loop_bindings, names),
        TemplateNodeKind::Text(parts) => collect_text_names(parts, loop_bindings, names),
        TemplateNodeKind::Conditional { condition, .. } => {
            read_optional_names(condition.as_ref(), loop_bindings, names);
        }
        TemplateNodeKind::Switch { value, .. } => read_names(value, loop_bindings, names),
        TemplateNodeKind::Case { when, .. } => {
            read_optional_names(when.as_ref(), loop_bindings, names);
        }
        TemplateNodeKind::Iteration {
            binding,
            iterable,
            children,
        } => {
            read_names(iterable, loop_bindings, names);
            collect_loop_body_names(binding, children, loop_bindings, names);
            return;
        }
        TemplateNodeKind::CountedIteration {
            binding,
            from,
            to,
            step,
            children,
            ..
        } => {
            for expression in [from, to, step] {
                read_names(expression, loop_bindings, names);
            }
            collect_loop_body_names(binding, children, loop_bindings, names);
            return;
        }
        TemplateNodeKind::Comment(_) | TemplateNodeKind::Slot => {}
    }
    if let Some(children) = node.kind.children() {
        collect_names_in_scope(children, loop_bindings, names);
    }
}

fn collect_text_names(
    parts: &[TextPart],
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    for part in parts {
        if let TextPart::Interpolation(expression, _) = part {
            read_names(expression, loop_bindings, names);
        }
    }
}

fn collect_loop_body_names(
    binding: &str,
    children: &[TemplateNode],
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    let mut inner_bindings = loop_bindings.clone();
    inner_bindings.insert(String::from(binding));
    collect_names_in_scope(children, &inner_bindings, names);
}

fn collect_attribute_names(
    attributes: &BTreeMap<String, TemplateAttribute>,
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    for attribute in attributes.values() {
        match &attribute.value {
            AttributeValue::Dynamic(expression) => read_names(expression, loop_bindings, names),
            AttributeValue::Event(binding) => collect_event_names(binding, loop_bindings, names),
            AttributeValue::Static(_) | AttributeValue::Control(_) => {}
        }
    }
}

fn collect_event_names(
    binding: &EventBinding,
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    if let Some((service, _)) = binding.handler.split_once('.') {
        names.insert(String::from(service));
    }
    if let Some(assignment) = &binding.assign
        && let Some((service, _)) = assignment.target.split_once('.')
    {
        names.insert(String::from(service));
    }
    for argument in &binding.arguments {
        if let EventArgument::Scope(expression) = argument {
            read_names(expression, loop_bindings, names);
        }
    }
}

fn read_optional_names(
    expression: Option<&Expression>,
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    if let Some(expression) = expression {
        read_names(expression, loop_bindings, names);
    }
}

fn read_names(
    expression: &Expression,
    loop_bindings: &BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    let Ok(value) = expression_value(expression) else {
        return;
    };
    let mut found = BTreeSet::new();
    json_identifiers(&value, &mut found);
    for name in found {
        if !loop_bindings.contains(&name) {
            names.insert(name);
        }
    }
}

fn json_identifiers(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if map.get("k").and_then(Value::as_str) == Some("id")
                && let Some(name) = map.get("n").and_then(Value::as_str)
            {
                names.insert(String::from(name));
            }
            for child in map.values() {
                json_identifiers(child, names);
            }
        }
        Value::Array(items) => {
            for item in items {
                json_identifiers(item, names);
            }
        }
        _ => {}
    }
}

fn data_references(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(name) = shared_data_member(map) {
                names.insert(String::from(name));
            }
            for child in map.values() {
                data_references(child, names);
            }
        }
        Value::Array(items) => {
            for item in items {
                data_references(item, names);
            }
        }
        _ => {}
    }
}

fn shared_data_member(map: &serde_json::Map<String, Value>) -> Option<&str> {
    let reads_a_member = map.get("k").and_then(Value::as_str) == Some("get");
    let of_shared_data = map.get("o").is_some_and(is_shared_data_root);
    if !reads_a_member || !of_shared_data {
        return None;
    }
    map.get("p").and_then(Value::as_str)
}

fn is_shared_data_root(value: &Value) -> bool {
    let is_identifier = value.get("k").and_then(Value::as_str) == Some("id");
    let name = value.get("n").and_then(Value::as_str);
    is_identifier && name == Some(ClientViewRenderer::SHARED_DATA_ROOT)
}

fn contains_slot(nodes: &[TemplateNode]) -> bool {
    for node in nodes {
        if matches!(node.kind, TemplateNodeKind::Slot) {
            return true;
        }
        if node.kind.children().is_some_and(contains_slot) {
            return true;
        }
    }
    false
}

fn client_failure(node: &TemplateNode, code: u16, message: &str) -> Failure {
    Failure::one(diagnostic(
        code,
        message,
        None,
        source_span(&node.source_path, node.range.start, node.range.end),
    ))
}

fn missing_data_failure(program: &TemplateProgram, missing: &str) -> Failure {
    Failure::one(diagnostic(
        1311,
        format!(
            "The view reads data.{missing}, but client/shared/data/{missing}.json does not exist."
        ),
        Some(String::from(
            "Add the JSON file, or correct the name: `data.<name>` reads client/shared/data/<name>.json.",
        )),
        source_span(&program.source_path, 0, 0),
    ))
}

fn serialization_failure(program: &TemplateProgram, error: &serde_json::Error) -> Failure {
    Failure::one(diagnostic(
        1305,
        format!("Cannot serialize the Tac client view: {error}"),
        None,
        source_span(&program.source_path, 0, 0),
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_data_tests {
    use super::*;

    fn identifiers_in(view: &str) -> BTreeSet<String> {
        let program = super::super::TemplateFrontend::compile(
            view,
            "client/components/demo/tac.html",
            &BTreeSet::new(),
        )
        .expect("view");
        let mut names = BTreeSet::new();
        collect_identifiers(&program.nodes, &mut names);
        names
    }

    #[test]
    fn a_loop_binding_is_a_local_not_a_service() {
        let names = identifiers_in(
            "<loop :for=\"topic of documentation.topics()\">\n  <a :href=\"topic.path\">{topic.title}</a>\n</loop>\n",
        );
        assert!(names.contains("documentation"), "{names:?}");
        assert!(!names.contains("topic"), "{names:?}");
        let names = identifiers_in(
            "<loop :for=\"topic of documentation.topics()\">{topic.title}</loop>\n<p>{topic.title}</p>\n",
        );
        assert!(names.contains("topic"), "{names:?}");
        let names = identifiers_in(
            "<loop :for=\"let page = 0; page < pager.last; page++\">{page}</loop>\n",
        );
        assert!(
            names.contains("pager") && !names.contains("page"),
            "{names:?}"
        );
    }

    #[test]
    fn a_handler_inside_a_loop_still_names_its_service() {
        let names = identifiers_in(
            "<loop :for=\"clipboard of snippet.files()\">\n  <button on:click=\"clipboard.copy(clipboard.name)\">Copy</button>\n</loop>\n",
        );
        assert!(
            names.contains("clipboard") && names.contains("snippet"),
            "{names:?}"
        );
    }

    #[test]
    fn only_a_named_member_of_data_is_a_reference() {
        let plan = json!([
            {"k": "get", "o": {"k": "id", "n": "data"}, "p": "faq"},
            {"k": "get", "o": {"k": "get", "o": {"k": "id", "n": "data"}, "p": "languages"}, "p": "tac"},
            {"k": "get", "o": {"k": "id", "n": "faq"}, "p": "entries"},
            {"k": "id", "n": "data"},
        ]);
        let mut read = BTreeSet::new();
        data_references(&plan, &mut read);
        assert_eq!(
            read,
            BTreeSet::from([String::from("faq"), String::from("languages")])
        );
    }
}
