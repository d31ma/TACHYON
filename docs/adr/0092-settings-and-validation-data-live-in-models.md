# ADR0092: Settings and validation data live in models

Status: accepted design; adapter qualification in progress.

The explicit role guard exposed historical settings and validation data classes
under helpers. These declarations describe data; they do not receive a Helper
lifecycle or a Service marker. Their generated test copies move to models,
while the historical conformance sources remain immutable.

Models may carry declarative Settings, field Setting bindings, Checked schemas,
Required, NotEmpty, Length, Min and Max field constraints, and Description
metadata. Native annotations remain native. The existing behavior parser checks
schema placement and literal arguments. Other framework roles and behavior
annotations remain refused, as do methods and arbitrary constructors.

A C# schema class may retain a private, parameterless constructor whose entire
body throws Exception or System.Exception with a plain string literal. This
preserves the existing no-instantiation conformance fixture. Interpolated or
computed arguments, assignments, method calls and other constructor bodies do
not qualify. Metadata lowering reads declarations and never constructs these
classes; SDK output retains the authored constructor.

Yon composes authored schema declarations before behavior lowering. Lowering
captures the configuration and validation metadata, then removes the temporary
data declarations. The existing model projection emits each model once, with
consumed schema annotations removed, retaining imports, namespace ownership and
native type declarations. Rust, C#, Kotlin, Python and PHP consumers use their
existing model namespaces through explicit imports. Java and Groovy remain in
their existing flattened compilation unit, Go remains package main, and Ruby
models are ordinary inline constants.

The approved recovery scope is the affected Yon languages and preservation of
the existing Java Tac checked-model adapter. Other compiled Tac schema adapters
remain pending; model capture reports a located TY1310 instead of accepting a
schema and silently dropping its behavior. Plain client models remain supported.
JavaScript and TypeScript model capture are implemented and qualified separately
by the coordinator using their existing frontend.

Acceptance requires the immutable settings and validation fixtures to execute
through their actual language adapters, including invalid inputs and the
throwing-constructor assertion. Parser and source-staging checks alone do not
establish runtime support. The first focused nine-language data-layout probe
found the expected C# constructor gap; the adapter and namespace projections
still require the complete targeted qualification batch.
