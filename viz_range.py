import matplotlib.pyplot as plt
import re

def parse_ranges(input_string):
    pattern = r'\(([-\d.]+),\s*([-\d.]+)\):\s*\(([-\d.]+),\s*([-\d.]+)\)'
    matches = re.findall(pattern, input_string)
    return [tuple(map(float, match)) for match in matches]

def visualize_mappings(input_string):
    mappings = parse_ranges(input_string)
    
    plt.figure(figsize=(8, 6))
    for a, b, c, d in mappings:
        plt.plot([a, b], [c, d], marker='o', label=f'({a}, {b}) → ({c}, {d})')
    
    plt.xlabel("Input Range")
    plt.ylabel("Output Range")
    plt.title("Function Range Mappings")
    # plt.legend()
    plt.grid(True)
    plt.show()

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) != 2:
        print("Usage: python script.py <filename>")
        sys.exit(1)
    
    filename = sys.argv[1]
    try:
        with open(filename, 'r') as file:
            input_string = file.read()
        visualize_mappings(input_string)
    except FileNotFoundError:
        print(f"Error: File '{filename}' not found.")
        sys.exit(1)

