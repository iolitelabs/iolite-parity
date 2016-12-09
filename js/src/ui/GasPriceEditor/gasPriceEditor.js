// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import BigNumber from 'bignumber.js';
import React, { Component, PropTypes } from 'react';
import { observer } from 'mobx-react';

import Input from '../Form/Input';
import GasPriceSelector from './GasPriceSelector';
import Store from './store';

import styles from './gasPriceEditor.css';

@observer
export default class GasPriceEditor extends Component {
  static propTypes = {
    children: PropTypes.node,
    store: PropTypes.object.isRequired,
    onChange: PropTypes.func
  }

  static Store = Store;

  render () {
    const { children, store } = this.props;
    const { estimated, priceDefault, price, gas, histogram, errorGas, errorPrice } = store;

    const gasLabel = `gas (estimated: ${new BigNumber(estimated).toFormat()})`;
    const priceLabel = `price (current: ${new BigNumber(priceDefault).toFormat()})`;

    return (
      <div className={ styles.columns }>
        <div className={ styles.graphColumn }>
          <GasPriceSelector
            gasPriceHistogram={ histogram }
            gasPrice={ price }
            onChange={ this.onEditGasPrice } />
          <div className={ styles.gasPriceDesc }>
            You can choose the gas price based on the
            distribution of recent included transaction gas prices.
            The lower the gas price is, the cheaper the transaction will
            be. The higher the gas price is, the faster it should
            get mined by the network.
          </div>
        </div>

        <div className={ styles.editColumn }>
          <div className={ styles.row }>
            <Input
              label={ gasLabel }
              hint='the amount of gas to use for the transaction'
              error={ errorGas }
              value={ gas }
              onChange={ this.onEditGas } />

            <Input
              label={ priceLabel }
              hint='the price of gas to use for the transaction'
              error={ errorPrice }
              value={ price }
              onChange={ this.onEditGasPrice } />
          </div>

          <div className={ styles.row }>
            { children }
          </div>
        </div>
      </div>
    );
  }

  onEditGas = (event, gas) => {
    const { store, onChange } = this.props;

    store.setGas(gas);
    onChange && onChange('gas', gas);
  }

  onEditGasPrice = (event, price) => {
    const { store, onChange } = this.props;

    store.setPrice(price);
    onChange && onChange('gasPrice', price);
  }
}
