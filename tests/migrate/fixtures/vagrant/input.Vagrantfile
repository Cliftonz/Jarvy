Vagrant.configure("2") do |config|
  config.vm.box = "ubuntu/jammy64"
  config.vm.network "forwarded_port", guest: 3000, host: 3000
  config.vm.network "forwarded_port", guest: 5432, host: 5432
  config.vm.synced_folder ".", "/home/vagrant/app"

  config.vm.provision "shell", inline: <<-SHELL
    apt-get update
    apt-get install -y nodejs npm postgresql
    sudo -u vagrant -i bash -c 'cd /home/vagrant/app && npm ci'
  SHELL
end
