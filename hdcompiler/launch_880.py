import os
def compile_file(filename):
    compiler_path = "./880btg/compiler_.py"
    asm_path = f"./880btg_ropchain/{filename}.asm"
    if not os.path.exists(asm_path):
        print(f"Tệp '{asm_path}' không tồn tại.")
        return
    command = f'python {compiler_path} -f hex < {asm_path}'
    os.system(command)
def main():
    while True:
        os.system('cls' if os.name == 'nt' else 'clear')
        filename = input("Nhập tên của tệp (chỉ dành cho 880 BTG): ").strip()
        compile_file(filename)
        print("\nĐã hoàn thành")
        input()

if __name__ == "__main__":
    main()