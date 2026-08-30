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
                
                # We want to replace emmm_lib::modules::<mod_name>::application:: to emmm_lib::modules::<mod_name>::api::
                # and similarly for domain::
                
                # regex to match modules::\w+::application::
                content = re.sub(r'(modules::\w+)::application::', r'\1::api::', content)
                content = re.sub(r'(modules::\w+)::domain::', r'\1::api::', content)
                
                with open(filepath, "w") as f:
                    f.write(content)

replace_in_tests()
print("Replaced application/domain with api in tests.")
