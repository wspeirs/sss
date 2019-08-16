# sss
The Simple Shell Scripting language is designed to run programs, and manipulate their output easily. It's an alternative to a Bash (shell) script, and more specifically designed towards executing programs than something generic like Python or Perl. `sss` is implemented in Rust.

# Language Overview

I think the best way to learn a new language is by example, so here are a few:

### Constants
`const x:num = 123.456; // constant number`
`const y:str = "hello world"; // constant string literal`

`ARG` is a special constant that is built-in and is of type `str:[]`. Each array element contains the arguments passed to the script, zero indexed.

### Variables
```
var a:str = "hello"; // create a string variable, and assign to string literal
var b:num = 23.4; // create a number variable, and assign number
var c:str[] = ["hello", "world"]; // create an array variable, assign literal
```

Variables are defined with the `var` keyword and are typed in the Rust fashion of `name:type`. There are only 3 types in `sss`:
* `num` - Any type of number: integer, floating point, etc
* `str` - Any type of string
* `pipe` - A pipe that results from running the built-in `run` command

`CWD = "/path/to/current/directory";`

`CWD` is a special variable that can be set or read, and represents the current working directory. `CWD` is automatically set to the directory the script was run from at the start of the script.

### Running Programs
The main point of the language is executing other programs and manipulating their output, including the return code. Programs are executed via the built-in `run` command. There are 2 formats for this command:

`run("/path/to/program arg1 arg2");` - Runs a program passing the full path, and arguments as a string
`run(["/path/to/program", ["arg1"], ["arg2"]);` - Run a program by passing an array of strings, the first containing the program, the rest the arguments.

The built-in run function returns a named tuple making it easy to access the exit code, STDOUT, and STDERR:
`var (exit_code:num, stdout:pipe, stderr:pipe) = run("/path/to/program");`

You can access any of the items of the tuple via their name if you don't need to manipulate them all: `var exit_code:num = run("/path/to/program").exit_code`

### Pipes
Pipes are a special variable type, they cannot be created directly, only from running commands. They are used to control the input and output of a command, and can be chained together in interesting ways:
* `+` read line-by-line the first pipe until EOF, then read the next one
* `zip` interleave line-by-line two pipes, continuing to read whichever pipe is longer

You can use the `.run` method on any pipe to send it as input to another program: 
```
var out_a:pipe = run("/path/to/program_a").stdout;
var out_b:pipe = run("/path/to/program_b").stdout;

(out_a + out_b).run("/path/to/program_c").exit_code
```
