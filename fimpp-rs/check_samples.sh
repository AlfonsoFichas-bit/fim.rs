#!/usr/bin/env bash
# Compara la salida del port en Rust con las salidas esperadas de los tests de Go
# (fim/main_test.go: TestReports).
set -u

BIN="$(dirname "$0")/target/debug/fimpp"
SAMPLES="$(dirname "$0")/../fim/samples"

pass=0
fail=0

check() {
  local name="$1" expected="$2" stdin_data="${3-}"
  local actual
  actual="$(printf '%s' "$stdin_data" | "$BIN" "$SAMPLES/$name" 2>/tmp/fim_err)"
  local code=$?
  if [ "$code" -ne 0 ]; then
    echo "FAIL $name (exit $code): $(head -2 /tmp/fim_err | tr '\n' ' ')"
    fail=$((fail + 1))
    return
  fi
  if [ "$actual" == "$expected" ]; then
    echo "ok   $name"
    pass=$((pass + 1))
  else
    echo "FAIL $name"
    echo "  esperado: $(printf '%s' "$expected" | od -c | head -3 | tr '\n' ' ')"
    echo "  obtenido: $(printf '%s' "$actual" | od -c | head -3 | tr '\n' ' ')"
    fail=$((fail + 1))
  fi
}

# Sin prompt
check array.fim              "$(printf 'Banana Cake\nGala\n')"
check brainfuck.fim          "Hello World!"
check bubblesort.fim         "$(printf '1\n2\n3\n4\n5\n7\n7\n')"
check deadfish.fim           "Hello world"
check digitalroot.fim        "$(printf '9\n')"
check eratosthenes.fim       "$(printf '2\n3\n5\n7\n11\n13\n17\n19\n')"
check factorial.fim          "$(printf '6\n24\n120\n720\n')"
check fibonacci.fim          "$(printf '34\n')"
check hello.fim              "$(printf 'Hello World!\n')"
check insertionsort.fim      "$(printf '1\n2\n3\n4\n5\n7\n7\n')"
check parameters.fim         "$(printf 'x\n1\ny\n0\n')"
check quicksort.fim          "$(printf '1\n2\n3\n4\n5\n7\n7\n')"
check recursion.fim          "$(printf '5\n4\n3\n2\n1\n')"
check string_index.fim       "$(printf 'T\nw\n')"
check sum.fim                "$(printf '5051\n')"

# Con prompt: el intérprete imprime el texto del prompt a stdout (igual que
# el CLI de Go), así que la salida esperada es un sufijo de la obtenida.
check_suffix() {
  local name="$1" expected="$2" stdin_data="$3"
  local actual
  actual="$(printf '%s' "$stdin_data" | "$BIN" "$SAMPLES/$name" 2>/tmp/fim_err)"
  local code=$?
  if [ "$code" -ne 0 ]; then
    echo "FAIL $name (exit $code): $(head -2 /tmp/fim_err | tr '\n' ' ')"
    fail=$((fail + 1))
    return
  fi
  case "$actual" in
    *"$expected")
      echo "ok   $name (prompt: ${actual%"$expected"})"
      pass=$((pass + 1));;
    *)
      echo "FAIL $name (sufijo no coincide)"
      echo "  esperado sufijo: $(printf '%s' "$expected" | od -c | head -3 | tr '\n' ' ')"
      echo "  obtenido:        $(printf '%s' "$actual" | od -c | head -3 | tr '\n' ' ')"
      fail=$((fail + 1));;
  esac
}

check_suffix input.fim              "$(printf 'Applejack said: Awesome!\n')"        "Awesome!"
check_suffix rot13.fim              "$(printf 'The ciphered text is: Uryyb Jbeyq!\nThe deciphered text is: Hello World!\n')" "Hello World!"
check_suffix squareroot.fim         "$(printf '7.071067984011346\n')"             "50"
check_suffix truth_machine.fim      "$(printf '0\n')"                             "0"

echo
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
