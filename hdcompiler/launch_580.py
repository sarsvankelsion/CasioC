import os
def compile_file(filename):
    compiler_path = "./580vnx/compiler_.py"
    asm_path = f"./580vnx_ropchain/{filename}.asm"
    if not os.path.exists(asm_path):
        print(f"Sai tên file hoặc có đâu ba :v")
        return
    command = f'python {compiler_path} -f hex < {asm_path}'
    os.system(command)
def main():
    while True:
        os.system('cls' if os.name == 'nt' else 'clear')
        filename = input("Compiler xin chào.Nổ cho tao tên file (Khác model 580 tao chịu XD): ").strip()
        compile_file(filename)
        print("\nXong rồi đó cha nội, thích nữa thì bấm phím nhắc tao :)")
        print("\nKhông thì đi ra ở đây chi :v")
        input()

if __name__ == "__main__":
    main()