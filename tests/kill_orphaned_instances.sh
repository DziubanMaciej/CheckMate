# TODO use non-default ports for tests
ps -aux | grep check_mate | grep "-p 10005" | awk '{print $2}' | xargs kill
ps -aux | grep check_mate | grep "-p 10005"
