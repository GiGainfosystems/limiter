#!/usr/bin/env bash

user_name="limiter"
prog_name="limiter"
service_name="limiter"

if id "$user_name" >/dev/null 2>&1; then
        ## user exists
        sleep 1;
else
        ## user does not exist
        adduser --system --group $user_name
        ## user needs sudo/su privilege,
fi

cp $prog_name /home/$user_name/
chmod +x /home/$user_name/$prog_name
cp .env /home/$user_name/
cp limits.json /home/$user_name/
chown -R $user_name:$user_name /home/$user_name/

cp $service_name.service /lib/systemd/system

systemctl enable $service_name.service;