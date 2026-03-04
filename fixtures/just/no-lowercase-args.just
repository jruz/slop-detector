build target:
    echo {{ target }}

deploy host port:
    echo {{ host }} {{ port }}

run *args:
    echo {{ args }}

start +services:
    echo {{ services }}

setup $env:
    echo $env

greet name="world":
    echo {{ name }}

test +flags="-q":
    echo {{ flags }}

serve $host="localhost" $port="8080":
    echo $host $port
