build TARGET:
    echo {{ TARGET }}

deploy HOST PORT:
    echo {{ HOST }} {{ PORT }}

run *ARGS:
    echo {{ ARGS }}

start +SERVICES:
    echo {{ SERVICES }}

setup $ENV:
    echo $ENV

greet NAME="world":
    echo {{ NAME }}

test +FLAGS="-q":
    echo {{ FLAGS }}

serve $HOST="localhost" $PORT="8080":
    echo $HOST $PORT
