import json
def getDistributed():
    # Used this query in https://api-kusama.interlay.io/gateway-graphql/console/api/api-explorer
    # query MyQuery {
    #   substrate_event(where: {name: {_eq: "vaultStaking.DistributeReward"}}) {
    #     data
    #   }
    # }
    with open('distribute-rewards.json') as json_file:
        data = json.load(json_file)
    
        # Initialize an empty totalDistributed map
        vaults = [str(event['data']['param1']['value']) for event in data['data']['substrate_event']]
        totalDistributed = {}
        for vault in vaults:
            totalDistributed[vault] = {'KINT': 0, 'KBTC': 0}

        for event in [e['data'] for e in data['data']['substrate_event']]:
            vaultId = str(event['param1']['value'])
            rewardCurrency = event['param0']['value']['token']
            reward = int(event['param2']['value'], 16)
            totalDistributed[vaultId][rewardCurrency] += reward
        return totalDistributed

def getWithdrawn():
    # Used this query in https://api-kusama.interlay.io/gateway-graphql/console/api/api-explorer
    # query MyQuery {
    #   substrate_event(where: {name: {_eq: "vaultStaking.WithdrawReward"}}) {
    #     data
    #   }
    # }
    with open('withdraw-rewards.json') as json_file:
        data = json.load(json_file)
    
        # initialize totalWithdrawn map
        vaults = [str(e['data']['param2']['value']) for e in data['data']['substrate_event']]
        totalWithdrawn = {}
        for vault in vaults:
            totalWithdrawn[vault] = {'KINT': 0, 'KBTC': 0, 'DOT': 0, 'KSM': 0}

        for event in [e['data'] for e in data['data']['substrate_event']]:
            vaultId = str(event['param2']['value'])
            rewardCurrency = event['param1']['value']['token']
            reward = int(str(event['param4']['value']), 16)
            totalWithdrawn[vaultId][rewardCurrency] += reward
        return totalWithdrawn

totalDistributed = getDistributed()
totalWithdrawn = getWithdrawn()

totalToSendKint = 0
for vault in totalDistributed.keys():
    for currency in totalDistributed[vault].keys():
        # vault id contained some metadata: strip out the useful parts
        vaultId = json.loads(vault.replace("'", '"'));
        accountId = vaultId['accountId'];
        collateral = vaultId['currencies']['collateral']['token'];
        wrapped = vaultId['currencies']['wrapped']['token'];
        amount = (totalDistributed[vault][currency] - totalWithdrawn[vault][currency]) / (10**18);
        print("Deficit:", accountId, collateral, wrapped, currency, int(amount))
        if currency == 'KINT':
            totalToSendKint += int(amount)

print("Total to send kint:", totalToSendKint)
