import json as _tac_json
import math as _tac_math

class _TacPythonCodec:
    MAX_BYTES = 1048576
    MAX_DEPTH = 32
    MAX_ITEMS = 4096
    MAX_INTEGER = 9007199254740991

    def __init__(self, types, schemas):
        self.types = types
        self.schemas = schemas

    def read(self, shape, value):
        self._bounded(value)
        return self._read(shape, value, 0)

    def plain(self, value):
        result = self._plain(value, 0)
        self._bounded(result)
        return result

    def result(self, shape, value):
        result = self._result(shape, value, 0)
        self._bounded(result)
        return result

    def _result(self, shape, value, depth):
        self._depth(depth)
        if type(shape) is str:
            return self._primitive(shape, value)
        return self._structured_result(shape, value, depth)

    def _structured_result(self, shape, value, depth):
        kind, inner = next(iter(shape.items()))
        handlers = {'optional': self._optional_result, 'list': self._list_result,
                    'model': self._model_result}
        return handlers[kind](inner, value, depth + 1)

    def _optional_result(self, inner, value, depth):
        if value is None:
            return None
        return self._result(inner, value, depth)

    def _list_result(self, inner, value, depth):
        self._require_list(value)
        return [self._result(inner, item, depth) for item in value]

    def _model_result(self, name, value, depth):
        schema = self.schemas[name]
        if schema.get('type') != 'object':
            return self._result(self._shape(schema), value, depth)
        return self._record_result(name, value, depth)

    def _record_result(self, name, value, depth):
        self._require_model(name, value)
        fields = self.schemas[name]['properties']
        return {field: self._result(self._shape(schema), self._field_value(value, field), depth)
                for field, schema in fields.items()}

    def _require_model(self, name, value):
        if type(value) is not dict and type(value) is not self.types.get(name):
            raise ValueError('Python Tac result does not match its declared model.')

    def _field_value(self, value, name):
        if type(value) is dict:
            return value.get(name)
        return getattr(value, name, None)

    def _bounded(self, value):
        encoded = _tac_json.dumps(value, allow_nan=False, ensure_ascii=False)
        if len(encoded.encode('utf-8')) > self.MAX_BYTES:
            raise ValueError('Python Tac JSON exceeds byte bounds.')

    def _depth(self, depth):
        if depth > self.MAX_DEPTH:
            raise ValueError('Python Tac JSON exceeds depth bounds.')

    def _read(self, shape, value, depth):
        self._depth(depth)
        if type(shape) is str:
            return self._primitive(shape, value)
        return self._structured(shape, value, depth)

    def _primitive(self, shape, value):
        handlers = {
            'any': self.plain,
            'text': self._text,
            'integer': self._integer,
            'number': self._number,
            'boolean': self._boolean,
        }
        return handlers[shape](value)

    def _text(self, value):
        if type(value) is not str:
            raise ValueError('Python Tac requires text.')
        return value

    def _integer(self, value):
        if type(value) is not int or abs(value) > self.MAX_INTEGER:
            raise ValueError('Python Tac requires a safe whole number.')
        return value

    def _number(self, value):
        if type(value) is int:
            return self._integer(value)
        return self._finite_float(value)

    def _finite_float(self, value):
        if type(value) is not float or not _tac_math.isfinite(value):
            raise ValueError('Python Tac requires a finite number.')
        return value

    def _boolean(self, value):
        if type(value) is not bool:
            raise ValueError('Python Tac requires true or false.')
        return value

    def _structured(self, shape, value, depth):
        kind, inner = next(iter(shape.items()))
        handlers = {'optional': self._optional, 'list': self._list, 'model': self._model}
        return handlers[kind](inner, value, depth + 1)

    def _optional(self, inner, value, depth):
        if value is None:
            return None
        return self._read(inner, value, depth)

    def _list(self, inner, value, depth):
        self._require_list(value)
        return [self._read(inner, item, depth) for item in value]

    def _require_list(self, value):
        if type(value) is not list or len(value) > self.MAX_ITEMS:
            raise ValueError('Python Tac requires a bounded list.')

    def _model(self, name, value, depth):
        schema = self.schemas[name]
        if schema.get('type') != 'object':
            return self._schema_read(schema, value, depth)
        return self._record((name, schema), value, depth)

    def _record(self, declared, value, depth):
        name, schema = declared
        self._require_record(schema, value)
        fields = {field: self._schema_read(schema['properties'][field], item, depth)
                  for field, item in value.items()}
        return self.types[name](**fields)

    def _require_record(self, schema, value):
        if type(value) is not dict or len(value) > self.MAX_ITEMS:
            raise ValueError('Python Tac requires a bounded model object.')
        self._record_keys(schema, value)

    def _record_keys(self, schema, value):
        fields = schema['properties']
        valid = set(value).issubset(fields) and set(schema.get('required', [])).issubset(value)
        if not valid:
            raise ValueError('Python Tac model has unknown or missing fields.')

    def _schema_read(self, schema, value, depth):
        return self._read(self._shape(schema), value, depth)

    def _shape(self, schema):
        if '$ref' in schema:
            return {'model': schema['$ref'].rsplit('/', 1)[-1]}
        return self._schema_shape(schema)

    def _schema_shape(self, schema):
        if 'anyOf' in schema:
            return {'optional': self._shape(schema['anyOf'][0])}
        return self._scalar_shape(schema)

    def _scalar_shape(self, schema):
        if schema.get('type') == 'array':
            return {'list': self._shape(schema['items'])}
        return {'string': 'text', 'integer': 'integer', 'number': 'number',
                'boolean': 'boolean'}.get(schema.get('type'), 'any')

    def _plain(self, value, depth):
        self._depth(depth)
        handlers = {str: self._text, int: self._integer, float: self._number, bool: self._boolean}
        if type(value) in handlers:
            return handlers[type(value)](value)
        return self._plain_container(value, depth)

    def _plain_container(self, value, depth):
        if value is None:
            return None
        return self._plain_collection(value, depth)

    def _plain_collection(self, value, depth):
        if type(value) is list:
            self._require_list(value)
            return [self._plain(item, depth + 1) for item in value]
        return self._plain_object(value, depth)

    def _plain_object(self, value, depth):
        if type(value) is dict:
            return self._plain_dictionary(value, depth)
        return self._plain_model(value, depth)

    def _plain_dictionary(self, value, depth):
        self._dictionary_keys(value)
        return {key: self._plain(item, depth + 1) for key, item in value.items()}

    def _dictionary_keys(self, value):
        valid = len(value) <= self.MAX_ITEMS and all(type(key) is str for key in value)
        if not valid:
            raise ValueError('Python Tac requires bounded string-keyed JSON objects.')

    def _plain_model(self, value, depth):
        name = type(value).__name__
        if self.types.get(name) is not type(value):
            raise ValueError('Python Tac result is not a declared JSON model.')
        return {field: self._plain(getattr(value, field), depth + 1)
                for field in self.schemas[name]['properties']}
