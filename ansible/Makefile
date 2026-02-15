VAULT_FILE=inventories/production/group_vars/all/vault.yml
VAULT_EXAMPLE=inventories/production/group_vars/all/vault.yml.example

.PHONY: vault-init vault-edit vault-view run ping

vault-init:
	cp $(VAULT_EXAMPLE) $(VAULT_FILE)
	ansible-vault encrypt $(VAULT_FILE)

vault-edit:
	ansible-vault edit $(VAULT_FILE)

vault-view:
	ansible-vault view $(VAULT_FILE)

ping:
	ansible all -m ping --ask-vault-pass

run:
	ansible-playbook site.yml --ask-vault-pass
