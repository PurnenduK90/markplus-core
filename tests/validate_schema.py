import sys
import json
from jsonschema import validate, ValidationError

def main():
    ast = json.loads(sys.stdin.read())
    with open("schema/markplus-ast.v1.schema.json") as f:
        schema = json.load(f)
    
    try:
        validate(instance=ast, schema=schema)
        print("✓ AST validates against schema/markplus-ast.v1.schema.json")
    except ValidationError as e:
        print(f"✗ Schema validation failed: {e.message}")
        print(f"  at path: {list(e.absolute_path)}")
        sys.exit(1)

if __name__ == "__main__":
    main()
