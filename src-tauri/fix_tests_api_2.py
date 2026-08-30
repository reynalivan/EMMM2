import os
import re

def replace_in_tests():
    test_dir = "tests"
    if not os.path.exists(test_dir): return
    
    for root, _, files in os.walk(test_dir):
        for file in files:
            if file.endswith(".rs"):
                filepath = os.path.join(root, file)
                with open(filepath, "r") as f:
                    content = f.read()
                
                content = re.sub(r'(modules::\w+)::application', r'\1::api::testing::application', content)
                content = re.sub(r'(modules::\w+)::domain', r'\1::api::testing::domain', content)
                content = re.sub(r'(modules::\w+)::adapters', r'\1::api::testing::adapters', content)
                
                with open(filepath, "w") as f:
                    f.write(content)

replace_in_tests()
print("Replaced with testing backdoor in tests.")
